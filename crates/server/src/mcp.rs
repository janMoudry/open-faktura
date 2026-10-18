//! Local stdio adapter. Every data call revalidates the session and membership.
use faktura_application::{
    auth::AuthStore,
    company::{self, CompanyError},
};
use faktura_infrastructure::{auth::PgAuth, company::PgCompany};
use rmcp::{model::*, service::RequestContext, ErrorData as McpError, RoleServer, ServerHandler};
use sqlx::PgPool;
#[derive(Clone)]
pub struct CompanyMcp {
    auth: PgAuth,
    store: PgCompany,
    token: String,
    rates: std::sync::Arc<dyn faktura_application::exchange_rate::RateProvider>,
    ares: faktura_infrastructure::ares::Ares,
    drafts: faktura_infrastructure::draft::PgDraft,
    contacts: faktura_infrastructure::contact::PgContact,
    catalog: faktura_infrastructure::catalog::PgCatalog,
    sales: faktura_infrastructure::sales::PgSales,
    delivery: faktura_infrastructure::delivery::PgDelivery,
    shares: faktura_infrastructure::share::PgShare,
    invoice_delivery: faktura_infrastructure::invoice_delivery::PgInvoiceDelivery,
}
impl CompanyMcp {
    pub fn with_rate_provider(
        mut self,
        provider: std::sync::Arc<dyn faktura_application::exchange_rate::RateProvider>,
    ) -> Self {
        self.rates = provider;
        self
    }
    pub fn new(pool: PgPool, token: String) -> Self {
        Self {
            auth: PgAuth::new(pool.clone(), None),
            drafts: faktura_infrastructure::draft::PgDraft(pool.clone()),
            contacts: faktura_infrastructure::contact::PgContact(pool.clone()),
            catalog: faktura_infrastructure::catalog::PgCatalog(pool.clone()),
            sales: faktura_infrastructure::sales::PgSales(pool.clone()),
            delivery: faktura_infrastructure::delivery::PgDelivery {
                pool: pool.clone(),
                mail: None,
            },
            shares: faktura_infrastructure::share::PgShare {
                pool: pool.clone(),
                base_url: None,
            },
            invoice_delivery: faktura_infrastructure::invoice_delivery::PgInvoiceDelivery {
                pool: pool.clone(),
                mail: None,
            },
            store: PgCompany(pool),
            token,
            ares: Default::default(),
            rates: std::sync::Arc::new(faktura_infrastructure::exchange_rate::Cnb::default()),
        }
    }
    pub fn with_delivery(mut self, store: faktura_infrastructure::delivery::PgDelivery) -> Self {
        self.invoice_delivery = faktura_infrastructure::invoice_delivery::PgInvoiceDelivery {
            pool: store.pool.clone(),
            mail: store.mail.clone(),
        };
        self.delivery = store;
        self
    }
    pub fn with_share(mut self, store: faktura_infrastructure::share::PgShare) -> Self {
        self.shares = store;
        self
    }
    async fn actor(&self) -> Result<String, McpError> {
        self.auth
            .session(&self.token)
            .await
            .map(|u| u.id)
            .map_err(|_| McpError::invalid_request("unauthorized", None))
    }
}
fn tools() -> Vec<Tool> {
    let empty = serde_json::json!({"type":"object","properties":{},"additionalProperties":false});
    let company = serde_json::json!({"type":"object","properties":{"company_id":{"type":"string"}},"required":["company_id"],"additionalProperties":false});
    let mut result = vec![
        Tool::new(
            "companies_list",
            "List only the authenticated user's company memberships",
            empty.as_object().unwrap().clone(),
        )
        .with_annotations(ToolAnnotations::new().read_only(true)),
        Tool::new(
            "company_context",
            "Read one authorized company, settings and role; select explicitly by company_id",
            company.as_object().unwrap().clone(),
        )
        .with_annotations(ToolAnnotations::new().read_only(true)),
    ];
    let communication: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/communication.tools.json"))
            .expect("communication tools");
    for schema in communication.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile invoice communication. Save can authorize automatic external email to customers. Preview is read-only. Pause stops future automated delivery for one invoice.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(matches!(name,"communication_settings"|"communication_preview"))));
    }
    let recurring: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/recurring.tools.json"))
            .expect("recurring tools");
    for schema in recurring.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped recurring invoices. Create is paused. Activate authorizes the saved automatic issue and email settings. Run generates one due occurrence. Pause does not cancel already queued messages.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(matches!(name,"recurring_list"|"recurring_get"|"recurring_preview"))));
    }
    let bank: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/bank.tools.json"))
            .expect("bank tools");
    for schema in bank.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped CZK banking with explicit foreign invoice allocation. Foreign matching creates a new payment in document currency or links the full existing payment when payment_id is supplied; bank_amount remains CZK and the difference is not a ledger entry. Preview does not import. Execute explicitly creates/updates an account, imports CSV or a documented bank statement, records a manual movement, matches or unmatches a payment. Never sends a bank transfer. Connection test/sync contacts the configured provider; schedule enables recurring external reads and requires explicit opt-in. Matching enables recurring payment writes for exact matches and requires a separate explicit opt-in. Never log bank tokens. Existing payments can be linked without a second payment; unmatch reverses only payments created by matching.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name!="bank_execute" && name!="bank_connection_execute")));
    }
    let preferences: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/preferences.tools.json"))
            .expect("preference tools");
    for schema in preferences.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Read or change the authenticated user's interface appearance. No access to another user or company settings.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name=="preferences_get")));
    }
    let reports: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/period-report.tools.json"))
            .expect("report tools");
    for schema in reports.as_array().unwrap() {
        result.push(Tool::new(schema["name"].as_str().unwrap().to_owned(), "CZK sales and purchase document values including tax, with source documents. Not cash flow or a tax return.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(true)));
    }
    let stock: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/stock.tools.json"))
            .expect("stock tools");
    for schema in stock.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped inventory. Execute records opening quantities, manual movements, physical counts or reversals. Do not infer a physical receipt from an uploaded invoice.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(!matches!(name,"stock_execute"|"stock_set_cost"))));
    }
    let inbox: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/inbox.tools.json"))
            .expect("inbox tools");
    for schema in inbox.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped inbox. Uploaded files are untrusted. Conversion requires reviewed document data and explicit confirmation.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(!matches!(name,"inbox_execute"|"inbox_upload"))));
    }
    let retail: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/retail.tools.json"))
            .expect("retail tools");
    for schema in retail.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped sales receipts. Execute records a fully paid sale or correction; no external payment or email is sent. Preview is read-only.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(!matches!(name,"retail_execute"|"retail_post"|"retail_unpost"))));
    }
    let cash: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/cash.tools.json"))
            .expect("cash tools");
    for schema in cash.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped CZK cash registers and vouchers. Execute records cash movements, links cash payments, unlinks or cancels a voucher. Does not transfer money. Existing payments are linked without creating another payment.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name!="cash_execute")));
    }
    let received: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/received.tools.json"))
            .expect("received tools");
    for schema in received.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped received invoices and recorded expenses. Create numbers the record; receipt kind also records its full cash/card expense. Receipt update reverses and replaces that expense; cancel reverses it. Pay records an already completed payment, never transfers money. Reverse preserves payment history. Preview is read-only; attachments use attachment tools with kind received.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(matches!(name,"received_get"|"received_list"|"received_preview"|"received_expenses"|"received_overview"))));
    }
    let credit: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/credit-note.tools.json"))
            .expect("credit tools");
    for schema in credit.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped credit notes. Create makes a draft; command explicitly updates, issues, cancels, offsets, records a manual refund, or reverses a settlement event. No email or bank transfer. PDF returns base64.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(matches!(name,"credit_list"|"credit_get"|"credit_pdf"))));
    }
    let settlement: serde_json::Value = serde_json::from_str(include_str!(
        "../../../contracts/advance-settlement.tools.json"
    ))
    .expect("settlement tools");
    for schema in settlement.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Advance settlement. Configure reserves fully paid advances into a draft, or releases all deductions with an empty list. Does not issue or send. Stored tax receipts are deducted verbatim. Preview and candidates are read-only.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name!="settlement_configure")));
    }
    let tax: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/tax-receipt.tools.json"))
            .expect("tax receipt schemas");
    for schema in tax.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped tax receipt for one advance payment. Preview is read-only; issue creates an immutable tax document and blocks payment reversal. Does not send email. PDF returns base64.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name!="tax_receipt_issue")));
    }
    let advance: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/advance.tools.json"))
            .expect("advance schemas");
    for schema in advance.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped proforma PDF and manual receipt history. Register/reverse change payment records, not tax documents or final invoices. PDF returns base64.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name=="advance_pdf"||name=="advance_payments")));
    }
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/export.tools.json"))
            .expect("export schema");
    result.push(Tool::new("register_export", "Read-only filtered invoices/contacts export. All matching rows (max 10000), selected columns, CSV/XLSX, CS/EN. Returns filename, media_type and base64. No records are changed.",schema[0]["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(true)));
    let schemas: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/contact.tools.json"))
            .expect("generated contact schemas");
    for schema in schemas.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(), "Profile-scoped contact directory. Writes require owner/editor; update also archives/restores with a revision.", schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name == "contacts_list" || name == "contact_get" || name == "contact_lookup_ares")));
    }
    let attachment_tools: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/attachment.tools.json"))
            .expect("attachment tools");
    for schema in attachment_tools.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped document files. Owner/editor uploads and deletes; viewers download. Historical downloads require an authorized delivery. Binary payload uses base64; maximum 2 MiB per file, 5 active files per document.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(!["attachment_upload","attachment_delete"].contains(&name))));
    }
    let smtp_tools: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/smtp.tools.json"))
            .expect("SMTP schemas");
    for schema in smtp_tools.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Installation administrator SMTP settings. smtp_test sends a real email: explicit recipient authorization required. Password is write-only; never log secrets.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name=="smtp_get")));
    }
    let invoice_mail: serde_json::Value = serde_json::from_str(include_str!(
        "../../../contracts/invoice-delivery.tools.json"
    ))
    .expect("invoice delivery schemas");
    for schema in invoice_mail.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped issued invoice email and history. invoice_send queues an actual email with PDF attachment or selected public link; use only with explicit authorization for the recipient. Issuing and payment are not changed. PDF returns base64.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name!="invoice_send")));
    }
    let schemas: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/catalog.tools.json"))
            .expect("generated catalog schemas");
    for schema in schemas.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(), "Profile-scoped catalog directory. Writes require owner/editor; update also archives/restores with a revision.", schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name == "catalogs_list" || name == "catalog_get" || name == "catalog_line")));
    }
    let schemas: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/sales.tools.json"))
            .expect("sales schemas");
    for schema in schemas.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped templates and quotations. sales_send queues an actual email with a PDF attachment or selected public share_id: use only with explicit authorization to send to the supplied recipient. sales_sharecreate grants anyone holding the returned URL read access to the current quotation until expiry or revocation. Conversion only saves a draft. PDF tools return base64.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name=="sales_list"||name=="sales_get"||name=="sales_pdf"||name=="sales_history"||name=="sales_deliverypdf"||name=="sales_sharelist"||name=="sales_shareviews")));
    }
    let rate_schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/exchange-rate.tools.json"))
            .expect("rate schema");
    result.push(
        Tool::new(
            "exchange_rate_lookup",
            "Read a CNB rate proposal for a profile; never changes documents.",
            rate_schema.as_object().unwrap().clone(),
        )
        .with_annotations(ToolAnnotations::new().read_only(true)),
    );
    let invoice_shares: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/invoice-share.tools.json"))
            .expect("invoice shares");
    for schema in invoice_shares.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped issued invoice links. Creation grants anyone holding the URL access to the issued PDF and current settlement status until expiry or revocation. Does not email. View counts are technical events, not proof of reading.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(matches!(name,"invoice_sharelist"|"invoice_shareviews"))));
    }
    let drafts: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/draft.tools.json"))
            .expect("draft schema");
    for schema in drafts.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(
            Tool::new(
                name.to_owned(),
                "Profile-scoped invoices. invoice_issue explicitly locks a saved revision and allocates a final number; invoice_pdf returns base64 PDF for issued invoices. No tool sends a document.",
                schema["inputSchema"].as_object().unwrap().clone(),
            )
            .with_annotations(ToolAnnotations::new().read_only(
                name == "invoices_list" || name == "invoice_get" || name == "invoice_pdf" || name == "invoice_payments" || name == "company_overview",
            )),
        );
    }
    let imports: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/import.tools.json"))
            .expect("import schemas");
    for schema in imports.as_array().unwrap() {
        let name = schema["name"].as_str().unwrap();
        result.push(Tool::new(name.to_owned(),"Profile-scoped historical import. Prepare persists a preview only; apply explicitly commits up to 25 records, repeated calls resume safely. Inspect errors and report; nothing is sent.",schema["inputSchema"].as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(name=="imports_list"||name=="import_get")));
    }
    let invoice: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/invoice.tools.json"))
            .expect("generated invoice schema");
    result.push(
        Tool::new(
            "invoice_calculate",
            "Calculate a versioned preview; does not save or issue an invoice.",
            invoice.as_object().unwrap().clone(),
        )
        .with_annotations(ToolAnnotations::new().read_only(true)),
    );
    result
}
impl ServerHandler for CompanyMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
    }
    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        self.actor().await?;
        Ok(ListToolsResult {
            tools: tools(),
            ..Default::default()
        })
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let actor = self.actor().await?;
        let mut args = request.arguments.unwrap_or_default();
        let result = match request.name.as_ref() {
            "import_money" => {
                let c = args
                    .remove("company_id")
                    .and_then(|v| v.as_str().map(str::to_owned));
                match (c, serde_json::from_value(serde_json::Value::Object(args))) {
                    (Some(c), Ok(input)) => {
                        faktura_infrastructure::import::PgImport(self.store.0.clone())
                            .prepare_money(&actor, &c, input)
                            .await
                            .and_then(|v| {
                                serde_json::to_value(v).map_err(|_| CompanyError::Unavailable)
                            })
                    }
                    _ => Err(CompanyError::Invalid(vec!["arguments".into()])),
                }
            }
            "imports_list" | "import_get" | "import_prepare" | "import_apply" => {
                import_tool(
                    &faktura_infrastructure::import::PgImport(self.drafts.0.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "company_overview"
            | "invoice_payments"
            | "invoice_register_payment"
            | "invoice_reverse_payment" => {
                payment_tool(
                    &faktura_infrastructure::payment::PgPayment(self.drafts.0.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "companies_list" if args.is_empty() => company::list(&self.store, &actor)
                .await
                .and_then(|v| serde_json::to_value(v).map_err(|_| CompanyError::Unavailable)),
            "company_context" if args.len() == 1 => match args
                .get("company_id")
                .and_then(|v| v.as_str())
            {
                Some(id) => company::context(&self.store, &actor, id)
                    .await
                    .and_then(|v| serde_json::to_value(v).map_err(|_| CompanyError::Unavailable)),
                None => Err(CompanyError::Invalid(vec!["company_id".into()])),
            },
            "contact_lookup_ares" if args.len() == 2 => {
                match (
                    args.get("company_id").and_then(|v| v.as_str()),
                    args.get("ico").and_then(|v| v.as_str()),
                ) {
                    (Some(company), Some(ico)) => faktura_application::ares::lookup(
                        &self.store,
                        &self.ares,
                        &actor,
                        company,
                        ico,
                    )
                    .await
                    .and_then(|v| serde_json::to_value(v).map_err(|_| CompanyError::Unavailable)),
                    _ => Err(CompanyError::Invalid(vec!["arguments".into()])),
                }
            }
            "invoices_list"
            | "invoice_get"
            | "invoice_create_draft"
            | "invoice_update_draft"
            | "invoice_issue"
            | "invoice_pdf" => draft_tool(&self.drafts, &actor, request.name.as_ref(), args).await,
            "exchange_rate_lookup" if args.len() == 3 => {
                match (
                    args.get("company_id").and_then(|v| v.as_str()),
                    args.get("currency").and_then(|v| v.as_str()),
                    args.get("date").and_then(|v| v.as_str()),
                ) {
                    (Some(c), Some(currency), Some(date)) => {
                        faktura_application::exchange_rate::lookup(
                            &self.store,
                            self.rates.as_ref(),
                            &actor,
                            c,
                            currency,
                            date,
                        )
                        .await
                        .and_then(|v| {
                            serde_json::to_value(v).map_err(|_| CompanyError::Unavailable)
                        })
                    }
                    _ => Err(CompanyError::Invalid(vec!["currency_or_date".into()])),
                }
            }
            "invoice_calculate" => {
                let mut args = args;
                let company = args
                    .remove("company_id")
                    .and_then(|v| v.as_str().map(str::to_owned));
                match (
                    company,
                    serde_json::from_value(serde_json::Value::Object(args)),
                ) {
                    (Some(company), Ok(input)) => faktura_application::invoice::calculate(
                        &self.store,
                        &actor,
                        &company,
                        input,
                    )
                    .await
                    .and_then(|v| serde_json::to_value(v).map_err(|_| CompanyError::Unavailable)),
                    _ => Err(CompanyError::Invalid(vec!["arguments".into()])),
                }
            }
            "communication_settings"
            | "communication_save"
            | "communication_preview"
            | "communication_pause" => {
                communication_tool(
                    &faktura_infrastructure::communication::PgCommunication(
                        self.delivery.pool.clone(),
                    ),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "recurring_create" | "recurring_command" | "recurring_list" | "recurring_get"
            | "recurring_preview" => {
                recurring_tool(
                    &faktura_infrastructure::recurring::PgRecurring(self.delivery.pool.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "preferences_get" | "preferences_set" => {
                preferences_tool(
                    &faktura_infrastructure::preferences::PgPreferences(self.store.0.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "tax_report" => {
                async {
                    use faktura_application::tax_report::TaxReportStore;
                    let mut args = args;
                    let c = args
                        .remove("company_id")
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .ok_or_else(|| CompanyError::Invalid(vec!["company_id".into()]))?;
                    let q = serde_json::from_value(serde_json::Value::Object(args))
                        .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))?;
                    let value =
                        faktura_infrastructure::tax_report::PgTaxReport(self.store.0.clone())
                            .report(&actor, &c, q)
                            .await?;
                    serde_json::to_value(value).map_err(|_| CompanyError::Unavailable)
                }
                .await
            }
            "dashboard_report" => {
                async {
                    use faktura_application::dashboard::DashboardStore;
                    let mut args = args;
                    let c = args
                        .remove("company_id")
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .ok_or_else(|| CompanyError::Invalid(vec!["company_id".into()]))?;
                    let q = serde_json::from_value(serde_json::Value::Object(args))
                        .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))?;
                    let value =
                        faktura_infrastructure::dashboard::PgDashboard(self.store.0.clone())
                            .dashboard(&actor, &c, q)
                            .await?;
                    serde_json::to_value(value).map_err(|_| CompanyError::Unavailable)
                }
                .await
            }
            "period_report" => {
                period_report_tool(
                    &faktura_infrastructure::period_report::PgPeriodReport(self.store.0.clone()),
                    &actor,
                    args,
                )
                .await
            }
            "item_sales_report" | "item_sales_export" => {
                item_sales_tool(
                    &faktura_infrastructure::item_sales::PgItemSales(self.store.0.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "stock_report" | "stock_set_cost" | "stock_costs" => {
                stock_report_tool(
                    &faktura_infrastructure::stock_report::PgStockReport(self.store.0.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "stock_cards" | "stock_movements" | "stock_execute" => {
                stock_tool(
                    &faktura_infrastructure::stock::PgStock(self.store.0.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "inbox_list" | "inbox_upload" | "inbox_execute" | "inbox_download"
            | "inbox_extract" | "inbox_history" => {
                inbox_tool(
                    &faktura_infrastructure::inbox::PgInbox(self.store.0.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "retail_list"
            | "retail_get"
            | "retail_history"
            | "retail_preview"
            | "retail_execute"
            | "retail_pdf"
            | "retail_postings"
            | "retail_posting_preview"
            | "retail_post"
            | "retail_unpost" => {
                retail_tool(
                    &faktura_infrastructure::retail::PgRetail(self.store.0.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "cash_terminals" | "cash_tax_preview" | "cash_settings" | "cash_pdf"
            | "cash_accounts" | "cash_movements" | "cash_get" | "cash_candidates"
            | "cash_balance" | "cash_execute" => {
                cash_tool(
                    &faktura_infrastructure::cash::PgCash(self.store.0.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "bank_balance"
            | "bank_accounts"
            | "bank_movements"
            | "bank_get"
            | "bank_candidates"
            | "bank_split_candidates"
            | "bank_foreign_candidates"
            | "bank_foreign_report"
            | "bank_preview"
            | "bank_statement_preview"
            | "bank_batch_preview"
            | "bank_suggestions"
            | "bank_execute" => {
                bank_tool(
                    &faktura_infrastructure::banking::PgBank(self.store.0.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "bank_connection_status" | "bank_connection_execute" => {
                async {
                    use faktura_application::bank_connection::ConnectionStore;
                    let c = args
                        .remove("company_id")
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .ok_or_else(|| CompanyError::Invalid(vec!["company_id".into()]))?;
                    let store = faktura_infrastructure::bank_connection::PgConnection::new(
                        self.store.0.clone(),
                    );
                    let result = if request.name == "bank_connection_status" {
                        let account = args
                            .remove("account_id")
                            .and_then(|v| v.as_str().map(str::to_owned))
                            .ok_or_else(|| CompanyError::Invalid(vec!["account_id".into()]))?;
                        if !args.is_empty() {
                            return Err(CompanyError::Invalid(vec!["arguments".into()]));
                        }
                        store.status(&actor, &c, &account).await?
                    } else {
                        let cmd = serde_json::from_value(serde_json::Value::Object(args))
                            .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))?;
                        store.execute(&actor, &c, cmd).await?
                    };
                    serde_json::to_value(result).map_err(|_| CompanyError::Unavailable)
                }
                .await
            }
            "received_expenses" | "received_overview" => {
                use faktura_application::purchase_report::PurchaseReportStore;
                let c = args
                    .remove("company_id")
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .ok_or_else(|| CompanyError::Invalid(vec!["company_id".into()]));
                match c {
                    Err(e) => Err(e),
                    Ok(c) => {
                        let store = faktura_infrastructure::purchase_report::PgPurchaseReport(
                            self.store.0.clone(),
                        );
                        let value = serde_json::Value::Object(args);
                        let invalid = |_| CompanyError::Invalid(vec!["arguments".into()]);
                        if request.name == "received_expenses" {
                            match serde_json::from_value(value).map_err(invalid) {
                                Ok(q) => store.payments(&actor, &c, q).await.and_then(|v| {
                                    serde_json::to_value(v).map_err(|_| CompanyError::Unavailable)
                                }),
                                Err(e) => Err(e),
                            }
                        } else {
                            match serde_json::from_value(value).map_err(invalid) {
                                Ok(q) => store.overview(&actor, &c, q).await.and_then(|v| {
                                    serde_json::to_value(v).map_err(|_| CompanyError::Unavailable)
                                }),
                                Err(e) => Err(e),
                            }
                        }
                    }
                }
            }
            "received_create" | "received_command" | "received_list" | "received_get"
            | "received_preview" => {
                received_tool(
                    &faktura_infrastructure::received::PgReceived(self.store.0.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "credit_create" | "credit_command" | "credit_list" | "credit_get" | "credit_pdf" => {
                credit_tool(
                    &faktura_infrastructure::credit_note::PgCredit(self.delivery.pool.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "settlement_configure" | "settlement_candidates" | "settlement_preview" => {
                settlement_tool(
                    &faktura_infrastructure::advance_settlement::PgSettlement(
                        self.delivery.pool.clone(),
                    ),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "tax_receipt_list"
            | "tax_receipt_get"
            | "tax_receipt_preview"
            | "tax_receipt_issue"
            | "tax_receipt_pdf" => {
                tax_receipt_tool(
                    &faktura_infrastructure::tax_receipt::PgTaxReceipt(self.delivery.pool.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "advance_pdf" | "advance_payments" | "advance_register" | "advance_reverse" => {
                advance_tool(
                    &faktura_infrastructure::advance::PgAdvance(self.delivery.pool.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "register_export" => {
                use base64::Engine;
                let mut args = args;
                let company = args
                    .remove("company_id")
                    .and_then(|v| v.as_str().map(str::to_owned));
                match (company,serde_json::from_value(serde_json::Value::Object(args))) {
                    (Some(c),Ok(input)) => faktura_application::export::run(&faktura_infrastructure::export::PgExport(self.delivery.pool.clone()),&actor,&c,input).await.map(|f|serde_json::json!({"filename":f.filename,"media_type":f.media_type,"base64":base64::engine::general_purpose::STANDARD.encode(f.bytes)})),
                    _=>Err(CompanyError::Invalid(vec!["arguments".into()]))
                }
            }
            "attachment_list"
            | "attachment_upload"
            | "attachment_download"
            | "attachment_delete"
            | "attachment_historydownload" => {
                attachment_tool(
                    &faktura_infrastructure::attachment::PgAttachment(self.delivery.pool.clone()),
                    &actor,
                    request.name.as_ref(),
                    args,
                )
                .await
            }
            "smtp_get" | "smtp_save" | "smtp_test" => {
                use faktura_application::smtp::{self, SmtpStore};
                let store = faktura_infrastructure::smtp::PgSmtp {
                    pool: self.delivery.pool.clone(),
                    fallback: self.delivery.mail.clone(),
                };
                let value = serde_json::Value::Object(args);
                match request.name.as_ref() {
                    "smtp_get" => store
                        .get(&actor)
                        .await
                        .map(|v| serde_json::to_value(v).unwrap()),
                    "smtp_save" => match serde_json::from_value(value) {
                        Ok(cmd) => smtp::save(&store, &actor, cmd)
                            .await
                            .map(|v| serde_json::to_value(v).unwrap()),
                        Err(_) => Err(CompanyError::Invalid(vec!["arguments".into()])),
                    },
                    _ => match serde_json::from_value(value) {
                        Ok(cmd) => smtp::test(&store, &actor, cmd)
                            .await
                            .map(|v| serde_json::to_value(v).unwrap()),
                        Err(_) => Err(CompanyError::Invalid(vec!["arguments".into()])),
                    },
                }
            }
            "invoice_send" | "invoice_history" | "invoice_deliverypdf" => {
                delivery_tool(&self.invoice_delivery, &actor, request.name.as_ref(), args).await
            }
            "sales_pdf" | "sales_deliverypdf" | "sales_send" | "sales_history" => {
                delivery_tool(&self.delivery, &actor, request.name.as_ref(), args).await
            }
            "invoice_sharecreate"
            | "invoice_sharelist"
            | "invoice_sharerevoke"
            | "invoice_shareviews" => {
                share_tool(
                    &faktura_infrastructure::invoice_share::PgInvoiceShare {
                        pool: self.shares.pool.clone(),
                        base_url: self.shares.base_url.clone(),
                    },
                    &actor,
                    &request.name.replace("invoice_", "sales_"),
                    args,
                )
                .await
            }
            "sales_sharecreate" | "sales_sharelist" | "sales_sharerevoke" | "sales_shareviews" => {
                share_tool(&self.shares, &actor, request.name.as_ref(), args).await
            }
            "sales_list" | "sales_get" | "sales_create" | "sales_update" | "sales_convert" => {
                sales_tool(&self.sales, &actor, request.name.as_ref(), args).await
            }
            "catalogs_list" | "catalog_get" | "catalog_create" | "catalog_update"
            | "catalog_line" => {
                catalog_tool(&self.catalog, &actor, request.name.as_ref(), args).await
            }
            "contacts_list" | "contact_get" | "contact_create" | "contact_update" => {
                contact_tool(&self.contacts, &actor, request.name.as_ref(), args).await
            }
            _ => {
                return Err(McpError::invalid_params(
                    "Unknown tool or invalid arguments",
                    None,
                ))
            }
        };
        Ok(match result {
            Ok(v) => CallToolResult::success(vec![ContentBlock::text(v.to_string())]),
            Err(e) => CallToolResult::error(vec![ContentBlock::text(match e {
                CompanyError::NotFound => "not_found",
                CompanyError::Forbidden => "forbidden",
                CompanyError::Invalid(_) => "invalid_input",
                CompanyError::Conflict => "revision_conflict",
                CompanyError::Unavailable => "unavailable",
            })]),
        }
        .into())
    }
}

async fn contact_tool(
    store: &faktura_infrastructure::contact::PgContact,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::contact;
    let company = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| CompanyError::Invalid(vec!["company_id".into()]))?;
    let id = if name == "contact_get" || name == "contact_update" {
        args.remove("id")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| CompanyError::Invalid(vec!["id".into()]))?
    } else {
        String::new()
    };
    fn parse<T: serde::de::DeserializeOwned>(
        args: serde_json::Map<String, serde_json::Value>,
    ) -> company::Result<T> {
        serde_json::from_value(serde_json::Value::Object(args))
            .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))
    }
    match name {
        "contacts_list" => {
            serde_json::to_value(contact::list(store, actor, &company, parse(args)?).await?)
        }
        "contact_get" if args.is_empty() => {
            serde_json::to_value(contact::get(store, actor, &company, &id).await?)
        }
        "contact_create" => {
            serde_json::to_value(contact::create(store, actor, &company, parse(args)?).await?)
        }
        "contact_update" => {
            serde_json::to_value(contact::update(store, actor, &company, &id, parse(args)?).await?)
        }
        _ => return Err(CompanyError::Invalid(vec!["arguments".into()])),
    }
    .map_err(|_| CompanyError::Unavailable)
}

async fn catalog_tool(
    store: &faktura_infrastructure::catalog::PgCatalog,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::catalog;
    let company = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| CompanyError::Invalid(vec!["company_id".into()]))?;
    let id = if name == "catalog_get" || name == "catalog_update" || name == "catalog_line" {
        args.remove("id")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| CompanyError::Invalid(vec!["id".into()]))?
    } else {
        String::new()
    };
    fn parse<T: serde::de::DeserializeOwned>(
        args: serde_json::Map<String, serde_json::Value>,
    ) -> company::Result<T> {
        serde_json::from_value(serde_json::Value::Object(args))
            .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))
    }
    match name {
        "catalogs_list" => {
            serde_json::to_value(catalog::list(store, actor, &company, parse(args)?).await?)
        }
        "catalog_get" if args.is_empty() => {
            serde_json::to_value(catalog::get(store, actor, &company, &id).await?)
        }
        "catalog_line" => {
            serde_json::to_value(catalog::line(store, actor, &company, &id, parse(args)?).await?)
        }
        "catalog_create" => {
            serde_json::to_value(catalog::create(store, actor, &company, parse(args)?).await?)
        }
        "catalog_update" => {
            serde_json::to_value(catalog::update(store, actor, &company, &id, parse(args)?).await?)
        }
        _ => return Err(CompanyError::Invalid(vec!["arguments".into()])),
    }
    .map_err(|_| CompanyError::Unavailable)
}

async fn draft_tool(
    store: &faktura_infrastructure::draft::PgDraft,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::draft;
    let company = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| CompanyError::Invalid(vec!["company_id".into()]))?;
    let id = if name == "invoice_get"
        || name == "invoice_update_draft"
        || name == "invoice_issue"
        || name == "invoice_pdf"
    {
        args.remove("id")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| CompanyError::Invalid(vec!["id".into()]))?
    } else {
        String::new()
    };
    fn parse<T: serde::de::DeserializeOwned>(
        args: serde_json::Map<String, serde_json::Value>,
    ) -> company::Result<T> {
        serde_json::from_value(serde_json::Value::Object(args))
            .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))
    }
    match name {
        "invoices_list" => {
            serde_json::to_value(draft::list(store, actor, &company, parse(args)?).await?)
        }
        "invoice_get" if args.is_empty() => {
            serde_json::to_value(draft::get(store, actor, &company, &id).await?)
        }
        "invoice_issue" => {
            serde_json::to_value(draft::issue(store, actor, &company, &id, parse(args)?).await?)
        }
        "invoice_pdf" if args.is_empty() => {
            use base64::Engine;
            let bytes = draft::pdf(store, actor, &company, &id).await?;
            Ok(serde_json::json!({"mime_type":"application/pdf", "base64":base64::engine::general_purpose::STANDARD.encode(bytes)}))
        }
        "invoice_create_draft" => {
            serde_json::to_value(draft::create(store, actor, &company, parse(args)?).await?)
        }
        "invoice_update_draft" => {
            serde_json::to_value(draft::update(store, actor, &company, &id, parse(args)?).await?)
        }
        _ => return Err(CompanyError::Invalid(vec!["arguments".into()])),
    }
    .map_err(|_| CompanyError::Unavailable)
}

async fn payment_tool(
    store: &faktura_infrastructure::payment::PgPayment,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::payment;
    fn take(
        args: &mut serde_json::Map<String, serde_json::Value>,
        key: &str,
    ) -> company::Result<String> {
        args.remove(key)
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| CompanyError::Invalid(vec![key.into()]))
    }
    fn parse<T: serde::de::DeserializeOwned>(
        args: serde_json::Map<String, serde_json::Value>,
    ) -> company::Result<T> {
        serde_json::from_value(serde_json::Value::Object(args))
            .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))
    }
    let company = take(&mut args, "company_id")?;
    if name == "company_overview" && args.is_empty() {
        return serde_json::to_value(payment::overview(store, actor, &company).await?)
            .map_err(|_| CompanyError::Unavailable);
    }
    let id = take(&mut args, "id")?;
    let result = match name {
        "invoice_payments" if args.is_empty() => payment::get(store, actor, &company, &id).await?,
        "invoice_register_payment" => {
            payment::register(store, actor, &company, &id, parse(args)?).await?
        }
        "invoice_reverse_payment" => {
            let payment = take(&mut args, "payment_id")?;
            payment::reverse(store, actor, &company, &id, &payment, parse(args)?).await?
        }
        _ => return Err(CompanyError::Invalid(vec!["arguments".into()])),
    };
    serde_json::to_value(result).map_err(|_| CompanyError::Unavailable)
}

async fn import_tool(
    store: &faktura_infrastructure::import::PgImport,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::import::{self as migration, ImportStore};
    let company = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| CompanyError::Invalid(vec!["company_id".into()]))?;
    let id = args
        .remove("id")
        .and_then(|v| v.as_str().map(str::to_owned));
    fn parse<T: serde::de::DeserializeOwned>(
        args: serde_json::Map<String, serde_json::Value>,
    ) -> company::Result<T> {
        serde_json::from_value(serde_json::Value::Object(args))
            .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))
    }
    match name {
        "imports_list" if args.is_empty() && id.is_none() => {
            serde_json::to_value(store.list(actor, &company).await?)
        }
        "import_get" if args.is_empty() && id.is_some() => {
            serde_json::to_value(store.get(actor, &company, id.as_deref().unwrap()).await?)
        }
        "import_prepare" if id.is_none() => {
            serde_json::to_value(migration::prepare(store, actor, &company, parse(args)?).await?)
        }
        "import_apply" if id.is_some() => serde_json::to_value(
            migration::apply(store, actor, &company, id.as_deref().unwrap(), parse(args)?).await?,
        ),
        _ => return Err(CompanyError::Invalid(vec!["arguments".into()])),
    }
    .map_err(|_| CompanyError::Unavailable)
}

async fn sales_tool(
    store: &faktura_infrastructure::sales::PgSales,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::sales;
    let company = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| CompanyError::Invalid(vec!["company_id".into()]))?;
    let id = if ["sales_get", "sales_update", "sales_convert"].contains(&name) {
        args.remove("id")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| CompanyError::Invalid(vec!["id".into()]))?
    } else {
        String::new()
    };
    fn parse<T: serde::de::DeserializeOwned>(
        args: serde_json::Map<String, serde_json::Value>,
    ) -> company::Result<T> {
        serde_json::from_value(serde_json::Value::Object(args))
            .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))
    }
    match name {
        "sales_list" => {
            serde_json::to_value(sales::list(store, actor, &company, parse(args)?).await?)
        }
        "sales_get" if args.is_empty() => {
            serde_json::to_value(sales::get(store, actor, &company, &id).await?)
        }
        "sales_create" => {
            serde_json::to_value(sales::create(store, actor, &company, parse(args)?).await?)
        }
        "sales_update" => {
            serde_json::to_value(sales::update(store, actor, &company, &id, parse(args)?).await?)
        }
        "sales_convert" => {
            serde_json::to_value(sales::convert(store, actor, &company, &id, parse(args)?).await?)
        }
        _ => return Err(CompanyError::Invalid(vec!["arguments".into()])),
    }
    .map_err(|_| CompanyError::Unavailable)
}

async fn delivery_tool(
    store: &dyn faktura_application::delivery::DeliveryStore,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::delivery;
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| CompanyError::Invalid(vec!["company_id".into()]))?;
    let id = args
        .remove("id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| CompanyError::Invalid(vec!["id".into()]))?;
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    match name {
        "invoice_send" => serde_json::to_value(
            faktura_application::invoice_delivery::send(
                store,
                actor,
                &c,
                &id,
                serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
            )
            .await?,
        )
        .map_err(|_| CompanyError::Unavailable),
        "sales_send" => serde_json::to_value(
            delivery::send(
                store,
                actor,
                &c,
                &id,
                serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
            )
            .await?,
        )
        .map_err(|_| CompanyError::Unavailable),
        "sales_history" | "invoice_history" => {
            let page = match args.remove("page") {
                Some(v) => v.as_i64().ok_or_else(invalid)?,
                None => 1,
            };
            if !args.is_empty() {
                return Err(invalid());
            }
            serde_json::to_value(delivery::history(store, actor, &c, &id, page).await?)
                .map_err(|_| CompanyError::Unavailable)
        }
        "sales_pdf" | "sales_deliverypdf" | "invoice_deliverypdf" => {
            let d = if name.ends_with("_deliverypdf") {
                Some(
                    args.remove("delivery")
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .ok_or_else(invalid)?,
                )
            } else {
                None
            };
            if !args.is_empty() {
                return Err(invalid());
            }
            use base64::Engine;
            Ok(
                serde_json::json!({"content_type":"application/pdf","base64":base64::engine::general_purpose::STANDARD.encode(delivery::pdf(store,actor,&c,&id,d.as_deref()).await?)}),
            )
        }
        _ => Err(invalid()),
    }
}

async fn share_tool(
    s: &dyn faktura_application::share::ShareStore,
    a: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::share;
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let d = args
        .remove("id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let result = match name {
        "sales_sharecreate" => serde_json::to_value(
            share::create(
                s,
                a,
                &c,
                &d,
                serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
            )
            .await?,
        ),
        "sales_sharelist" => {
            let p = match args.remove("page") {
                None => 1,
                Some(v) => v.as_i64().ok_or_else(invalid)?,
            };
            if !args.is_empty() {
                return Err(invalid());
            }
            serde_json::to_value(share::list(s, a, &c, &d, p).await?)
        }
        "sales_sharerevoke" | "sales_shareviews" => {
            let id = args
                .remove("share")
                .and_then(|v| v.as_str().map(str::to_owned))
                .ok_or_else(invalid)?;
            if name == "sales_sharerevoke" {
                if !args.is_empty() {
                    return Err(invalid());
                }
                share::revoke(s, a, &c, &d, &id).await?;
                Ok(serde_json::Value::Null)
            } else {
                let p = match args.remove("page") {
                    None => 1,
                    Some(v) => v.as_i64().ok_or_else(invalid)?,
                };
                if !args.is_empty() {
                    return Err(invalid());
                }
                serde_json::to_value(share::views(s, a, &c, &d, &id, p).await?)
            }
        }
        _ => return Err(invalid()),
    };
    result.map_err(|_| CompanyError::Unavailable)
}

async fn attachment_tool(
    store: &dyn faktura_application::attachment::AttachmentStore,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> Result<serde_json::Value, CompanyError> {
    use base64::Engine;
    use faktura_application::attachment;
    let mut take = |k: &str| {
        args.remove(k)
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| CompanyError::Invalid(vec![k.into()]))
    };
    let c = take("company_id")?;
    let kind = take("kind")?;
    let doc = take("document_id")?;
    let file = if !["attachment_list", "attachment_upload"].contains(&name) {
        Some(take("id")?)
    } else {
        None
    };
    let delivery = if name == "attachment_historydownload" {
        Some(take("delivery")?)
    } else {
        None
    };
    if name != "attachment_upload" && !args.is_empty() {
        return Err(CompanyError::Invalid(vec!["arguments".into()]));
    }
    match name {
        "attachment_list" => {
            Ok(serde_json::to_value(store.list(actor, &c, &kind, &doc).await?).unwrap())
        }
        "attachment_upload" => {
            let cmd = serde_json::from_value(serde_json::Value::Object(args))
                .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))?;
            Ok(
                serde_json::to_value(attachment::upload(store, actor, &c, &kind, &doc, cmd).await?)
                    .unwrap(),
            )
        }
        "attachment_delete" => {
            store
                .delete(actor, &c, &kind, &doc, file.as_deref().unwrap())
                .await?;
            Ok(serde_json::json!({"deleted":true}))
        }
        _ => {
            let f = store
                .download(
                    actor,
                    &c,
                    &kind,
                    &doc,
                    file.as_deref().unwrap(),
                    delivery.as_deref(),
                )
                .await?;
            Ok(
                serde_json::json!({"metadata":f.metadata,"base64":base64::engine::general_purpose::STANDARD.encode(f.content)}),
            )
        }
    }
}

async fn advance_tool(
    store: &dyn faktura_application::advance::AdvanceStore,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> faktura_application::company::Result<serde_json::Value> {
    use base64::Engine;
    use faktura_application::advance;
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let d = args
        .remove("id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    if name == "advance_pdf" || name == "advance_payments" {
        if !args.is_empty() {
            return Err(invalid());
        }
        return if name == "advance_pdf" {
            Ok(
                serde_json::json!({"base64":base64::engine::general_purpose::STANDARD.encode(store.pdf(actor,&c,&d).await?),"media_type":"application/pdf"}),
            )
        } else {
            serde_json::to_value(store.payments(actor, &c, &d).await?)
                .map_err(|_| CompanyError::Unavailable)
        };
    }
    if name == "advance_register" {
        advance::register(
            store,
            actor,
            &c,
            &d,
            serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
        )
        .await?;
    } else {
        let p = args
            .remove("payment")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(invalid)?;
        advance::reverse(
            store,
            actor,
            &c,
            &d,
            &p,
            serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
        )
        .await?;
    }
    Ok(serde_json::Value::Null)
}

async fn tax_receipt_tool(
    store: &dyn faktura_application::tax_receipt::TaxReceiptStore,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> faktura_application::company::Result<serde_json::Value> {
    use base64::Engine;
    use faktura_application::tax_receipt;
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let take = |args: &mut serde_json::Map<String, serde_json::Value>, key: &str| {
        args.remove(key)
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(invalid)
    };
    let c = take(&mut args, "company_id")?;
    let result = match name {
        "tax_receipt_issue" => serde_json::to_value(
            tax_receipt::issue(
                store,
                actor,
                &c,
                serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
            )
            .await?,
        ),
        "tax_receipt_list" => serde_json::to_value(
            tax_receipt::list(
                store,
                actor,
                &c,
                serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
            )
            .await?,
        ),
        "tax_receipt_preview" => {
            let d = take(&mut args, "advance_id")?;
            let p = take(&mut args, "payment_id")?;
            if !args.is_empty() {
                return Err(invalid());
            }
            serde_json::to_value(store.preview(actor, &c, &d, &p).await?)
        }
        _ => {
            let id = take(&mut args, "id")?;
            if !args.is_empty() {
                return Err(invalid());
            }
            if name == "tax_receipt_pdf" {
                return Ok(
                    serde_json::json!({"media_type":"application/pdf","base64":base64::engine::general_purpose::STANDARD.encode(store.pdf(actor,&c,&id).await?)}),
                );
            }
            serde_json::to_value(store.get(actor, &c, &id).await?)
        }
    };
    result.map_err(|_| CompanyError::Unavailable)
}

async fn settlement_tool(
    store: &dyn faktura_application::advance_settlement::SettlementStore,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> faktura_application::company::Result<serde_json::Value> {
    use faktura_application::advance_settlement;
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let result = match name {
        "settlement_configure" => serde_json::to_value(
            advance_settlement::configure(
                store,
                actor,
                &c,
                serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
            )
            .await?,
        ),
        "settlement_candidates" => serde_json::to_value(
            advance_settlement::candidates(
                store,
                actor,
                &c,
                serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
            )
            .await?,
        ),
        _ => {
            let id = args
                .remove("id")
                .and_then(|v| v.as_str().map(str::to_owned))
                .ok_or_else(invalid)?;
            serde_json::to_value(
                store
                    .preview(
                        actor,
                        &c,
                        &id,
                        serde_json::from_value(serde_json::Value::Object(args))
                            .map_err(|_| invalid())?,
                    )
                    .await?,
            )
        }
    };
    result.map_err(|_| CompanyError::Unavailable)
}

async fn credit_tool(
    store: &dyn faktura_application::credit_note::CreditStore,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> faktura_application::company::Result<serde_json::Value> {
    use faktura_application::credit_note;
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let result = match name {
        "credit_create" => serde_json::to_value(
            credit_note::create(
                store,
                actor,
                &c,
                serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
            )
            .await?,
        ),
        "credit_list" => serde_json::to_value(
            credit_note::list(
                store,
                actor,
                &c,
                serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
            )
            .await?,
        ),
        _ => {
            let id = args
                .remove("id")
                .and_then(|v| v.as_str().map(str::to_owned))
                .ok_or_else(invalid)?;
            if name == "credit_command" {
                serde_json::to_value(
                    credit_note::command(
                        store,
                        actor,
                        &c,
                        &id,
                        serde_json::from_value(serde_json::Value::Object(args))
                            .map_err(|_| invalid())?,
                    )
                    .await?,
                )
            } else if name == "credit_pdf" {
                use base64::Engine;
                Ok(
                    serde_json::json!({"mime_type":"application/pdf","content_base64":base64::engine::general_purpose::STANDARD.encode(store.pdf(actor,&c,&id).await?)}),
                )
            } else {
                serde_json::to_value(store.get(actor, &c, &id).await?)
            }
        }
    };
    result.map_err(|_| CompanyError::Unavailable)
}

async fn recurring_tool(
    store: &dyn faktura_application::recurring::RecurringStore,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> faktura_application::company::Result<serde_json::Value> {
    use faktura_application::recurring;
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let result = match name {
        "recurring_preview" => serde_json::to_value(
            store
                .preview(
                    actor,
                    &c,
                    serde_json::from_value(serde_json::Value::Object(args))
                        .map_err(|_| invalid())?,
                )
                .await?,
        ),
        "recurring_create" => serde_json::to_value(
            recurring::create(
                store,
                actor,
                &c,
                serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
            )
            .await?,
        ),
        "recurring_list" => serde_json::to_value(
            recurring::list(
                store,
                actor,
                &c,
                serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
            )
            .await?,
        ),
        _ => {
            let id = args
                .remove("id")
                .and_then(|v| v.as_str().map(str::to_owned))
                .ok_or_else(invalid)?;
            if name == "recurring_command" {
                serde_json::to_value(
                    recurring::command(
                        store,
                        actor,
                        &c,
                        &id,
                        serde_json::from_value(serde_json::Value::Object(args))
                            .map_err(|_| invalid())?,
                    )
                    .await?,
                )
            } else {
                serde_json::to_value(store.get(actor, &c, &id).await?)
            }
        }
    };
    result.map_err(|_| CompanyError::Unavailable)
}

async fn communication_tool(
    store: &dyn faktura_application::communication::CommunicationStore,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let v = match name {
        "communication_settings" if args.is_empty() => {
            serde_json::to_value(store.settings(actor, &c).await?)
        }
        "communication_save" => serde_json::to_value(
            store
                .save(
                    actor,
                    &c,
                    serde_json::from_value(serde_json::Value::Object(args))
                        .map_err(|_| invalid())?,
                )
                .await?,
        ),
        "communication_preview" | "communication_pause" => {
            let id = args
                .remove("id")
                .and_then(|v| v.as_str().map(str::to_owned))
                .ok_or_else(invalid)?;
            if name == "communication_pause" {
                serde_json::to_value(
                    store
                        .pause(
                            actor,
                            &c,
                            &id,
                            serde_json::from_value(serde_json::Value::Object(args))
                                .map_err(|_| invalid())?,
                        )
                        .await?,
                )
            } else {
                let purpose = args
                    .remove("purpose")
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .ok_or_else(invalid)?;
                if !args.is_empty() {
                    return Err(invalid());
                }
                serde_json::to_value(store.preview(actor, &c, &id, &purpose).await?)
            }
        }
        _ => return Err(invalid()),
    };
    v.map_err(|_| CompanyError::Unavailable)
}

async fn received_tool(
    store: &dyn faktura_application::received::ReceivedStore,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::received;
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let id = args
        .remove("id")
        .and_then(|v| v.as_str().map(str::to_owned));
    let value = serde_json::Value::Object(args);
    let result = match name {
        "received_create" => serde_json::to_value(
            received::create(
                store,
                actor,
                &c,
                serde_json::from_value(value).map_err(|_| invalid())?,
            )
            .await?,
        ),
        "received_command" => serde_json::to_value(
            received::command(
                store,
                actor,
                &c,
                &id.ok_or_else(invalid)?,
                serde_json::from_value(value).map_err(|_| invalid())?,
            )
            .await?,
        ),
        "received_get" => {
            serde_json::to_value(store.get(actor, &c, &id.ok_or_else(invalid)?).await?)
        }
        "received_preview" => serde_json::to_value(
            store
                .preview(
                    actor,
                    &c,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        _ => serde_json::to_value(
            store
                .list(
                    actor,
                    &c,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
    };
    result.map_err(|_| CompanyError::Unavailable)
}

async fn bank_tool(
    store: &dyn faktura_application::banking::BankStore,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let company = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let id = args.get("id").and_then(|v| v.as_str().map(str::to_owned));
    let value = serde_json::Value::Object(args);
    let result = match name {
        "bank_accounts" => serde_json::to_value(store.accounts(actor, &company).await?),
        "bank_balance" => serde_json::to_value(
            store
                .balance(
                    actor,
                    &company,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "bank_movements" => serde_json::to_value(
            store
                .movements(
                    actor,
                    &company,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "bank_get" => {
            serde_json::to_value(store.get(actor, &company, &id.ok_or_else(invalid)?).await?)
        }
        "bank_suggestions" => serde_json::to_value(
            store
                .suggestions(actor, &company, &id.ok_or_else(invalid)?)
                .await?,
        ),
        "bank_split_candidates" => serde_json::to_value(
            store
                .split_candidates(
                    actor,
                    &company,
                    &id.ok_or_else(invalid)?,
                    value["query"].as_str().unwrap_or_default(),
                )
                .await?,
        ),
        "bank_foreign_report" => serde_json::to_value(
            store
                .foreign_report(
                    actor,
                    &company,
                    serde_json::from_value(value.clone()).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "bank_foreign_candidates" => serde_json::to_value(
            store
                .foreign_candidates(
                    actor,
                    &company,
                    &id.ok_or_else(invalid)?,
                    value["query"].as_str().unwrap_or_default(),
                )
                .await?,
        ),
        "bank_candidates" => serde_json::to_value(
            store
                .candidates(
                    actor,
                    &company,
                    &id.ok_or_else(invalid)?,
                    value["query"].as_str().unwrap_or_default(),
                )
                .await?,
        ),
        "bank_batch_preview" => serde_json::to_value(
            store
                .batch_preview(
                    actor,
                    &company,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "bank_statement_preview" => serde_json::to_value(
            store
                .statement_preview(
                    actor,
                    &company,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "bank_preview" => serde_json::to_value(
            store
                .preview(
                    actor,
                    &company,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "bank_execute" => serde_json::to_value(
            store
                .execute(
                    actor,
                    &company,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        _ => return Err(invalid()),
    };
    result.map_err(|_| CompanyError::Unavailable)
}

async fn cash_tool(
    store: &dyn faktura_application::cash::CashStore,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let company = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let id = args.get("id").and_then(|v| v.as_str().map(str::to_owned));
    let value = serde_json::Value::Object(args);
    let result = match name {
        "cash_settings" => serde_json::to_value(store.settings(actor, &company).await?),
        "cash_tax_preview" => serde_json::to_value(
            store
                .tax_preview(
                    actor,
                    &company,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "cash_pdf" => {
            use base64::Engine;
            let bytes = store
                .pdf(
                    actor,
                    &company,
                    &id.ok_or_else(invalid)?,
                    value["language"].as_str().ok_or_else(invalid)?,
                )
                .await?;
            Ok(
                serde_json::json!({"mime_type":"application/pdf","filename":"cash-voucher.pdf","content_base64":base64::engine::general_purpose::STANDARD.encode(bytes)}),
            )
        }
        "cash_terminals" => serde_json::to_value(store.terminals(actor, &company).await?),
        "cash_accounts" => serde_json::to_value(store.accounts(actor, &company).await?),
        "cash_movements" => serde_json::to_value(
            store
                .movements(
                    actor,
                    &company,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "cash_get" => {
            serde_json::to_value(store.get(actor, &company, &id.ok_or_else(invalid)?).await?)
        }
        "cash_candidates" => serde_json::to_value(
            store
                .candidates(
                    actor,
                    &company,
                    &id.ok_or_else(invalid)?,
                    value["query"].as_str().unwrap_or_default(),
                )
                .await?,
        ),
        "cash_balance" => serde_json::to_value(
            store
                .balance(
                    actor,
                    &company,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "cash_execute" => serde_json::to_value(
            store
                .execute(
                    actor,
                    &company,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        _ => return Err(invalid()),
    };
    result.map_err(|_| CompanyError::Unavailable)
}

async fn retail_tool(
    store: &faktura_infrastructure::retail::PgRetail,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use faktura_application::retail::{PostingStore, RetailStore};
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_owned();
    let value = serde_json::Value::Object(args);
    let result = match name {
        "retail_postings" => serde_json::to_value(store.postings(actor, &c).await?),
        "retail_posting_preview" => serde_json::to_value(
            store
                .posting_preview(
                    actor,
                    &c,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "retail_post" => serde_json::to_value(
            store
                .post(
                    actor,
                    &c,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "retail_unpost" => serde_json::to_value(
            store
                .unpost(
                    actor,
                    &c,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "retail_list" => serde_json::to_value(
            store
                .list(
                    actor,
                    &c,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "retail_get" => serde_json::to_value(store.get(actor, &c, &id).await?),
        "retail_history" => serde_json::to_value(store.history(actor, &c, &id).await?),
        "retail_preview" => serde_json::to_value(
            store
                .preview(
                    actor,
                    &c,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "retail_execute" => serde_json::to_value(
            store
                .execute(
                    actor,
                    &c,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "retail_pdf" => {
            use base64::Engine;
            return Ok(
                serde_json::json!({"mime_type":"application/pdf","content_base64":base64::engine::general_purpose::STANDARD.encode(store.pdf(actor,&c,&id).await?)}),
            );
        }
        _ => return Err(invalid()),
    };
    result.map_err(|_| CompanyError::Unavailable)
}

async fn inbox_tool(
    store: &dyn faktura_application::inbox::InboxStore,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let value = serde_json::Value::Object(args);
    let result = match name {
        "inbox_history" => serde_json::to_value(
            store
                .history(actor, &c, value["id"].as_str().ok_or_else(invalid)?)
                .await?,
        ),
        "inbox_extract" => serde_json::to_value(
            store
                .extract(actor, &c, value["id"].as_str().ok_or_else(invalid)?)
                .await?,
        ),
        "inbox_list" => serde_json::to_value(
            store
                .list(
                    actor,
                    &c,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "inbox_upload" => serde_json::to_value(
            store
                .upload(
                    actor,
                    &c,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "inbox_execute" => serde_json::to_value(
            store
                .execute(
                    actor,
                    &c,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "inbox_download" => {
            use base64::Engine;
            let (d, bytes) = store
                .download(actor, &c, value["id"].as_str().ok_or_else(invalid)?)
                .await?;
            return Ok(
                serde_json::json!({"metadata":d,"content_base64":base64::engine::general_purpose::STANDARD.encode(bytes)}),
            );
        }
        _ => return Err(invalid()),
    };
    result.map_err(|_| CompanyError::Unavailable)
}

async fn stock_tool(
    store: &dyn faktura_application::stock::StockStore,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let value = serde_json::Value::Object(args);
    let result = match name {
        "stock_cards" => serde_json::to_value(store.cards(actor, &c).await?),
        "stock_movements" => serde_json::to_value(
            store
                .movements(
                    actor,
                    &c,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "stock_execute" => serde_json::to_value(
            store
                .execute(
                    actor,
                    &c,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        _ => return Err(invalid()),
    };
    result.map_err(|_| CompanyError::Unavailable)
}

async fn preferences_tool(
    store: &dyn faktura_application::preferences::PreferenceStore,
    actor: &str,
    name: &str,
    args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    let value = if name == "preferences_get" {
        store.get(actor).await?
    } else {
        store
            .set(
                actor,
                serde_json::from_value(serde_json::Value::Object(args))
                    .map_err(|_| CompanyError::Invalid(vec!["preferences".into()]))?,
            )
            .await?
    };
    serde_json::to_value(value).map_err(|_| CompanyError::Unavailable)
}

async fn period_report_tool(
    store: &dyn faktura_application::period_report::PeriodReportStore,
    actor: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let q = serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?;
    serde_json::to_value(store.report(actor, &c, q).await?).map_err(|_| CompanyError::Unavailable)
}

async fn stock_report_tool(
    store: &dyn faktura_application::stock_report::StockReportStore,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let value = serde_json::Value::Object(args);
    let r = match name {
        "stock_report" => serde_json::to_value(
            store
                .report(
                    actor,
                    &c,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "stock_set_cost" => serde_json::to_value(
            store
                .set_cost(
                    actor,
                    &c,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        ),
        "stock_costs" => serde_json::to_value(
            store
                .costs(actor, &c, value["catalog_id"].as_str().ok_or_else(invalid)?)
                .await?,
        ),
        _ => return Err(invalid()),
    };
    r.map_err(|_| CompanyError::Unavailable)
}

async fn item_sales_tool(
    store: &dyn faktura_application::item_sales::ItemSalesStore,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> company::Result<serde_json::Value> {
    use base64::Engine;
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let value = serde_json::Value::Object(args);
    if name == "item_sales_export" {
        let f = store
            .export(
                actor,
                &c,
                serde_json::from_value(value).map_err(|_| invalid())?,
            )
            .await?;
        Ok(
            serde_json::json!({"filename":f.filename,"media_type":f.media_type,"base64":base64::engine::general_purpose::STANDARD.encode(f.bytes)}),
        )
    } else {
        serde_json::to_value(
            store
                .report(
                    actor,
                    &c,
                    serde_json::from_value(value).map_err(|_| invalid())?,
                )
                .await?,
        )
        .map_err(|_| CompanyError::Unavailable)
    }
}
