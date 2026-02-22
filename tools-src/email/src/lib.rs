//! Email WASM Tool für IronClaw.
//! Sendet E-Mails über den lokalen SMTP-HTTP-Proxy.

wit_bindgen::generate!({
    world: "sandboxed-tool",
    path: "../../wit/tool.wit",
});

use serde::{Deserialize, Serialize};

struct EmailTool;

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum EmailAction {
    SendEmail {
        to: String,
        subject: String,
        body: String,
        #[serde(default)]
        cc: Option<String>,
    },
}

#[derive(Debug, Serialize)]
struct SendResult {
    ok: bool,
    message: String,
}

impl exports::near::agent::tool::Guest for EmailTool {
    fn execute(req: exports::near::agent::tool::Request) -> exports::near::agent::tool::Response {
        match execute_inner(&req.params) {
            Ok(result) => exports::near::agent::tool::Response { output: Some(result), error: None },
            Err(e) => exports::near::agent::tool::Response { output: None, error: Some(e) },
        }
    }

    fn schema() -> String {
        r#"{
            "type": "object",
            "required": ["action", "to", "subject", "body"],
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["send_email"],
                    "description": "Aktion: send_email"
                },
                "to": {
                    "type": "string",
                    "description": "Empfänger-E-Mail-Adresse"
                },
                "subject": {
                    "type": "string",
                    "description": "Betreff der E-Mail"
                },
                "body": {
                    "type": "string",
                    "description": "Inhalt der E-Mail (Plaintext)"
                },
                "cc": {
                    "type": "string",
                    "description": "CC-Empfänger, kommagetrennt (optional)"
                }
            }
        }"#.to_string()
    }

    fn description() -> String {
        "Sendet E-Mails von jarvis@isikon.net über den lokalen SMTP-Proxy. \
         Unterstützt To, Subject, Body und CC."
            .to_string()
    }
}

fn execute_inner(params: &str) -> Result<String, String> {
    let action: EmailAction =
        serde_json::from_str(params).map_err(|e| format!("Ungültige Parameter: {}", e))?;

    match action {
        EmailAction::SendEmail { to, subject, body, cc } => {
            let mut payload = serde_json::json!({
                "to": to,
                "subject": subject,
                "body": body,
            });
            if let Some(cc_val) = cc {
                payload["cc"] = serde_json::Value::String(cc_val);
            }

            let payload_bytes = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
            let headers = r#"{"Content-Type": "application/json"}"#;

            let response = crate::near::agent::host::http_request(
                "POST",
                "http://127.0.0.1:8025/send",
                headers,
                Some(&payload_bytes),
                None,
            )?;

            if response.status != 200 {
                return Err(format!("Proxy-Fehler: Status {}", response.status));
            }

            let result = serde_json::json!({
                "ok": true,
                "message": format!("E-Mail an {} gesendet.", to)
            });
            Ok(serde_json::to_string(&result).unwrap())
        }
    }
}

export!(EmailTool);
