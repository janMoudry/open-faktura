use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use async_trait::async_trait;
use faktura_application::auth::{AuthError, AuthResult, AuthStore, Identity};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::sync::{Arc, OnceLock};
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct PgAuth {
    pool: PgPool,
    bootstrap_hash: Option<[u8; 32]>,
}
fn digest(value: &str) -> [u8; 32] {
    Sha256::digest(value.as_bytes()).into()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn opaque() -> String {
    let mut bytes = [0; 32];
    OsRng.fill_bytes(&mut bytes);
    hex(&bytes)
}
fn unavailable(_: impl std::fmt::Display) -> AuthError {
    AuthError::Unavailable
}
async fn password_work(password: String, existing: Option<String>) -> AuthResult<String> {
    static LIMIT: OnceLock<Arc<Semaphore>> = OnceLock::new();
    let permit = LIMIT
        .get_or_init(|| Arc::new(Semaphore::new(4)))
        .clone()
        .try_acquire_owned()
        .map_err(|_| AuthError::RateLimited)?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let argon = Argon2::default();
        if let Some(hash) = existing {
            let parsed = PasswordHash::new(&hash).map_err(unavailable)?;
            argon
                .verify_password(password.as_bytes(), &parsed)
                .map_err(|_| AuthError::Unauthorized)?;
            Ok(hash)
        } else {
            let salt = SaltString::generate(&mut OsRng);
            Ok(argon
                .hash_password(password.as_bytes(), &salt)
                .map_err(unavailable)?
                .to_string())
        }
    })
    .await
    .map_err(unavailable)?
}
impl PgAuth {
    pub fn new(pool: PgPool, bootstrap_secret: Option<String>) -> Self {
        Self {
            pool,
            bootstrap_hash: bootstrap_secret.map(|v| digest(&v)),
        }
    }
    async fn throttle(&self, bucket: &str, max: i32) -> AuthResult<()> {
        let count: i32 = sqlx::query_scalar("INSERT INTO auth_rate_limit(bucket, attempts) VALUES ($1,1) ON CONFLICT(bucket) DO UPDATE SET attempts = CASE WHEN auth_rate_limit.started_at < now() - interval '15 minutes' THEN 1 ELSE auth_rate_limit.attempts + 1 END, started_at = CASE WHEN auth_rate_limit.started_at < now() - interval '15 minutes' THEN now() ELSE auth_rate_limit.started_at END RETURNING attempts")
            .bind(bucket).fetch_one(&self.pool).await.map_err(unavailable)?;
        if count > max {
            Err(AuthError::RateLimited)
        } else {
            Ok(())
        }
    }
}
#[async_trait]
impl AuthStore for PgAuth {
    async fn setup_required(&self) -> AuthResult<bool> {
        sqlx::query_scalar("SELECT completed_at IS NULL FROM installation_state WHERE singleton")
            .fetch_one(&self.pool)
            .await
            .map_err(unavailable)
    }
    async fn bootstrap(&self, secret: &str, email: &str, password: &str) -> AuthResult<()> {
        if !self.setup_required().await? {
            return Err(AuthError::AlreadyConfigured);
        }
        self.throttle("bootstrap", 20).await?;
        let expected = self.bootstrap_hash.ok_or(AuthError::Unavailable)?;
        if !bool::from(expected.ct_eq(&digest(secret))) {
            return Err(AuthError::Unauthorized);
        }
        let hash = password_work(password.into(), None).await?;
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        let pending: bool = sqlx::query_scalar(
            "SELECT completed_at IS NULL FROM installation_state WHERE singleton FOR UPDATE",
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(unavailable)?;
        if !pending {
            return Err(AuthError::AlreadyConfigured);
        }
        let id: i64 = sqlx::query_scalar("INSERT INTO app_user(email,password_hash,installation_admin) VALUES ($1,$2,true) RETURNING id")
            .bind(email).bind(hash).fetch_one(&mut *tx).await.map_err(unavailable)?;
        sqlx::query("UPDATE installation_state SET completed_at=now() WHERE singleton")
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
        sqlx::query("INSERT INTO installation_audit(user_id,action) VALUES ($1,'bootstrap')")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)
    }
    async fn login(&self, email: &str, password: &str) -> AuthResult<(Identity, String)> {
        self.throttle("login-global", 500).await?;
        self.throttle(&format!("login:{}", hex(&digest(email))), 10)
            .await?;
        let row = sqlx::query("SELECT id,name,email,password_hash,installation_admin,credential_version FROM app_user WHERE email=$1")
            .bind(email).fetch_optional(&self.pool).await.map_err(unavailable)?;
        let Some(row) = row else {
            // Perform the same expensive primitive even for an unknown account.
            password_work(password.into(), None).await?;
            return Err(AuthError::Unauthorized);
        };
        password_work(password.into(), Some(row.get("password_hash"))).await?;
        let id: i64 = row.get("id");
        let version: i64 = row.get("credential_version");
        let token = opaque();
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        // Serialize against recovery: a password checked before reset must not
        // produce a new valid session after all old sessions were revoked.
        let current: i64 =
            sqlx::query_scalar("SELECT credential_version FROM app_user WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_one(&mut *tx)
                .await
                .map_err(unavailable)?;
        if current != version {
            return Err(AuthError::Unauthorized);
        }
        sqlx::query("DELETE FROM auth_session WHERE expires_at <= now()")
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
        sqlx::query("DELETE FROM auth_rate_limit WHERE started_at < now() - interval '1 day'")
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
        sqlx::query("INSERT INTO auth_session(token_hash,user_id,expires_at) VALUES ($1,$2,now()+interval '12 hours')")
            .bind(hex(&digest(&token))).bind(id).execute(&mut *tx).await.map_err(unavailable)?;
        sqlx::query("INSERT INTO installation_audit(user_id,action) VALUES ($1,'login')")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        Ok((
            Identity {
                id: id.to_string(),
                email: row.get("email"),
                name: row.get("name"),
                installation_admin: row.get("installation_admin"),
            },
            token,
        ))
    }
    async fn session(&self, token: &str) -> AuthResult<Identity> {
        if token.len() != 64 {
            return Err(AuthError::Unauthorized);
        }
        let row = sqlx::query("SELECT u.id,u.name,u.email,u.installation_admin FROM auth_session s JOIN app_user u ON u.id=s.user_id WHERE token_hash=$1 AND expires_at>now()")
            .bind(hex(&digest(token))).fetch_optional(&self.pool).await.map_err(unavailable)?.ok_or(AuthError::Unauthorized)?;
        Ok(Identity {
            id: row.get::<i64, _>("id").to_string(),
            email: row.get("email"),
            name: row.get("name"),
            installation_admin: row.get("installation_admin"),
        })
    }
    async fn logout(&self, token: &str) -> AuthResult<()> {
        sqlx::query("DELETE FROM auth_session WHERE token_hash=$1")
            .bind(hex(&digest(token)))
            .execute(&self.pool)
            .await
            .map_err(unavailable)?;
        Ok(())
    }
    async fn update_account(
        &self,
        token: &str,
        name: &str,
        email: &str,
        current_password: &str,
        password: Option<&str>,
    ) -> AuthResult<Identity> {
        let identity = self.session(token).await?;
        self.throttle(&format!("account:{}", identity.id), 20)
            .await?;
        let id: i64 = identity.id.parse().map_err(unavailable)?;
        let row =
            sqlx::query("SELECT email,password_hash,credential_version FROM app_user WHERE id=$1")
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(unavailable)?;
        let version: i64 = row.get("credential_version");
        let sensitive = email != row.get::<String, _>("email") || password.is_some();
        if sensitive {
            password_work(current_password.into(), Some(row.get("password_hash")))
                .await
                .map_err(|e| {
                    if e == AuthError::Unauthorized {
                        AuthError::InvalidPassword
                    } else {
                        e
                    }
                })?;
        }
        let hash = if let Some(password) = password {
            Some(password_work(password.into(), None).await?)
        } else {
            None
        };
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        let current: i64 =
            sqlx::query_scalar("SELECT credential_version FROM app_user WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_one(&mut *tx)
                .await
                .map_err(unavailable)?;
        if current != version {
            return Err(AuthError::Conflict);
        }
        // Recheck the session under the user lock, serializing with recovery and other account edits.
        let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM auth_session WHERE token_hash=$1 AND user_id=$2 AND expires_at>now())")
            .bind(hex(&digest(token))).bind(id).fetch_one(&mut *tx).await.map_err(unavailable)?;
        if !valid {
            return Err(AuthError::Unauthorized);
        }
        sqlx::query("UPDATE app_user SET name=$2,email=$3,password_hash=COALESCE($4,password_hash),credential_version=credential_version+1 WHERE id=$1")
            .bind(id).bind(name).bind(email).bind(hash).execute(&mut *tx).await.map_err(|e| {
                if e.as_database_error().is_some_and(|e| e.is_unique_violation()) { AuthError::EmailTaken } else { unavailable(e) }
            })?;
        if sensitive {
            sqlx::query("DELETE FROM auth_session WHERE user_id=$1 AND token_hash<>$2")
                .bind(id)
                .bind(hex(&digest(token)))
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
        }
        sqlx::query("INSERT INTO installation_audit(user_id,action) VALUES ($1,$2)")
            .bind(id)
            .bind(if sensitive {
                "account_credentials_changed"
            } else {
                "account_updated"
            })
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        Ok(Identity {
            name: name.into(),
            email: email.into(),
            ..identity
        })
    }
    async fn recover(&self, email: &str, password: &str) -> AuthResult<()> {
        let hash = password_work(password.into(), None).await?;
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        let id: i64 = sqlx::query_scalar("UPDATE app_user SET password_hash=$2, credential_version=credential_version+1 WHERE email=$1 AND installation_admin RETURNING id")
            .bind(email).bind(hash).fetch_optional(&mut *tx).await.map_err(unavailable)?.ok_or(AuthError::Unauthorized)?;
        sqlx::query("DELETE FROM auth_session WHERE user_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
        sqlx::query("DELETE FROM auth_rate_limit WHERE bucket=$1")
            .bind(format!("login:{}", hex(&digest(email))))
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
        sqlx::query("INSERT INTO installation_audit(user_id,action) VALUES ($1,'admin_recovery')")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)
    }
}
