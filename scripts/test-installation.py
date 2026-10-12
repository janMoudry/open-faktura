"""Isolated Compose smoke: bootstrap, recovery, restart, backup and restore.
Requires the local image open-faktura:p02 and generated .secrets. Never uses
production volumes: both Compose project names are random and removed at exit.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
import uuid

root = Path(__file__).resolve().parent.parent
os.chdir(root)

def run(args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, **kwargs)

with tempfile.TemporaryDirectory(prefix="faktura-install-") as temporary:
    folder = Path(temporary)
    override = folder / "override.yaml"
    override.write_text('services:\n  app:\n    ports: ["127.0.0.1::3001"]\n')
    projects = ["p02test" + uuid.uuid4().hex[:12] for _ in range(2)]
    envfiles = []
    for project in projects:
        envfile = folder / (project + ".env")
        envfile.write_text(f"APP_DOMAIN=localhost\nAPP_VERSION=p02\nCOMPOSE_PROJECT_NAME={project}\n")
        envfiles.append(envfile)
    def compose(index, *args, **kwargs):
        return run(["docker", "compose", "--env-file", str(envfiles[index]), "-f", "infra/compose.production.yaml", "-f", str(override), *args], **kwargs)
    def address(index):
        port = compose(index, "port", "app", "3001").stdout.decode().strip().split(":")[-1]
        return "http://127.0.0.1:" + port
    def request(index, path, data=None, cookie=None):
        headers = {"Origin":"https://localhost", "X-Faktura-Request":"1"}
        if data is not None: headers["Content-Type"] = "application/json"
        if cookie: headers["Cookie"] = cookie
        req = urllib.request.Request(address(index)+path, data=None if data is None else json.dumps(data).encode(), headers=headers)
        try: response = urllib.request.urlopen(req, timeout=20)
        except urllib.error.HTTPError as error: response = error
        return response.status, response.headers, response.read()
    try:
        compose(0,"up","-d","--no-build","--wait","app")
        assert b"Uid:\t10001" in compose(0,"exec","-T","app","cat","/proc/1/status").stdout
        assert request(0,"/api/auth/setup")[2] == b'{"setup_required":true}'
        secret = (root/".secrets/bootstrap").read_text().strip()
        payload = {"email":"smoke@example.test","password":"Synthetic smoke password 123!","secret":secret}
        assert request(0,"/api/auth/setup",payload)[0] == 201
        status, headers, _ = request(0,"/api/auth/login",{k:payload[k] for k in ["email","password"]})
        assert status == 200
        assert "Secure" in headers["Set-Cookie"]
        cookie = headers["Set-Cookie"].split(";")[0]
        assert request(0,"/api/auth/session",cookie=cookie)[0] == 200
        compose(0,"restart","app")
        for _ in range(30):
            try:
                if request(0,"/api/auth/session",cookie=cookie)[0] == 200: break
            except (OSError, urllib.error.URLError): pass
            time.sleep(1)
        else: raise AssertionError("Session did not survive restart")
        compose(0,"exec","-T","app","faktura-server","recover-admin","smoke@example.test","-",input=b"Replacement smoke password 456!\n")
        assert request(0,"/api/auth/session",cookie=cookie)[0] == 401
        # exec bypasses the image entrypoint but the copied secret and DB URL file
        # remain available; the CLI reads them without exporting them to output.
        status, headers, _ = request(0,"/api/auth/login",{"email":payload["email"],"password":"Replacement smoke password 456!"})
        assert status == 200
        backup_cookie = headers["Set-Cookie"].split(";")[0]
        compose(0,"exec","-T","app","sh","-c","printf 'synthetic attachment' > /data/files/smoke.txt")
        backup = folder / "backup"
        run(["sh","scripts/backup.sh",str(backup)],env={**os.environ,"PRODUCTION_ENV_FILE":str(envfiles[0])})
        run(["sh","scripts/restore.sh",str(backup)],env={**os.environ,"PRODUCTION_ENV_FILE":str(envfiles[1])})
        compose(1,"up","-d","--no-build","--wait","app")
        assert request(1,"/api/auth/setup")[2] == b'{"setup_required":false}'
        assert request(1,"/api/auth/session",cookie=backup_cookie)[0] == 401
        assert request(1,"/api/auth/login",{"email":payload["email"],"password":"Replacement smoke password 456!"})[0] == 200
        assert compose(1,"exec","-T","app","cat","/data/files/smoke.txt").stdout == b"synthetic attachment"
        refused=subprocess.run(["sh","scripts/restore.sh",str(backup)],env={**os.environ,"PRODUCTION_ENV_FILE":str(envfiles[1])},capture_output=True)
        assert refused.returncode != 0
        assert b"target database is not empty" in refused.stderr
        print("PASS: production image, atomic setup, Secure cookie, restart, CLI recovery, DB/files backup and clean restore; sessions revoked on restore; nonempty restore refused.")
    finally:
        for index in range(2):
            compose(index,"down","--volumes","--remove-orphans")
