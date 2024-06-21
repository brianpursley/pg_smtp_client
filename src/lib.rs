use pgrx::prelude::*;

mod guc;

pg_module_magic!();

#[pg_guard]
pub extern "C" fn _PG_init() {
    guc::init();
}

mod smtp_client {
    use super::*;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::transport::smtp::client::{Tls, TlsParameters};
    use lettre::{Message, SmtpTransport, Transport};

    fn create_mailer(
        smtp_server: Option<&str>,
        smtp_port: Option<i32>,
        smtp_tls: Option<bool>,
        smtp_username: Option<&str>,
        smtp_password: Option<&str>,
    ) -> Result<SmtpTransport, String> {
        let Some(server) = smtp_server.map_or_else(guc::get_smtp_server, |x| Some(x.to_string()))
        else {
            return Err("SMTP server not provided and no default configured".to_string());
        };

        let port = smtp_port.map_or_else(guc::get_smtp_port, |x| x as u16);
        let tls = smtp_tls.unwrap_or_else(guc::get_smtp_tls);
        let username = smtp_username.map_or_else(guc::get_smtp_username, |x| Some(x.to_string()));
        let password = smtp_password.map_or_else(guc::get_smtp_password, |x| Some(x.to_string()));

        let mut mailer = SmtpTransport::relay(server.as_str())
            .map_err(|e| format!("Failed to create SMTP relay: {:?}", e))?
            .port(port);

        if tls {
            let tls_parameters = TlsParameters::new(server)
                .map_err(|e| format!("Failed to create TLS parameters: {:?}", e))?;
            mailer = mailer.tls(Tls::Wrapper(tls_parameters));
        } else {
            mailer = mailer.tls(Tls::None);
        }

        if let Some(u) = username {
            if let Some(p) = password {
                mailer = mailer.credentials(Credentials::new(u, p));
            } else {
                return Err(
                    "SMTP username provided without password and no default configured".to_string(),
                );
            }
        }

        Ok(mailer.build())
    }

    #[allow(clippy::too_many_arguments)]
    fn create_message(
        subject: &str,
        body: &str,
        html: bool,
        from_address: Option<&str>,
        recipients: Option<Vec<Option<&str>>>,
        ccs: Option<Vec<Option<&str>>>,
        bccs: Option<Vec<Option<&str>>>,
        keep_bcc_header: bool,
    ) -> Result<Message, String> {
        let mut email = Message::builder().subject(subject);

        if let Some(addr) = from_address
            .map(|x| x.to_string())
            .or_else(guc::get_smtp_from)
        {
            if let Ok(parsed_addr) = addr.parse() {
                email = email.from(parsed_addr);
            } else {
                return Err(format!("Invalid from: {}", addr));
            }
        } else {
            return Err("From address not provided and no default configured".to_string());
        }

        if let Some(items) = recipients {
            for addr in items.into_iter().flatten() {
                if let Ok(parsed_addr) = addr.parse() {
                    email = email.to(parsed_addr);
                } else {
                    return Err(format!("Invalid to: {}", addr));
                }
            }
        }

        if let Some(items) = ccs {
            for addr in items.into_iter().flatten() {
                if let Ok(parsed_addr) = addr.parse() {
                    email = email.cc(parsed_addr);
                } else {
                    return Err(format!("Invalid cc: {}", addr));
                }
            }
        }

        if let Some(items) = bccs {
            if keep_bcc_header {
                email = email.keep_bcc();
            }
            for addr in items.into_iter().flatten() {
                if let Ok(parsed_addr) = addr.parse() {
                    email = email.bcc(parsed_addr);
                } else {
                    return Err(format!("Invalid bcc: {}", addr));
                }
            }
        }

        if html {
            email = email.header(lettre::message::header::ContentType::TEXT_HTML);
        }

        email.body(body.to_string()).map_err(|e| e.to_string())
    }

    #[pg_extern]
    #[allow(clippy::too_many_arguments)]
    fn send_email(
        subject: &str,
        body: &str,
        html: default!(bool, "false"),
        from_address: default!(Option<&str>, "NULL"),
        recipients: default!(Option<Vec<Option<&str>>>, "NULL"),
        ccs: default!(Option<Vec<Option<&str>>>, "NULL"),
        bccs: default!(Option<Vec<Option<&str>>>, "NULL"),
        smtp_server: default!(Option<&str>, "NULL"),
        smtp_port: default!(Option<i32>, "NULL"),
        smtp_tls: default!(Option<bool>, "NULL"),
        smtp_username: default!(Option<&str>, "NULL"),
        smtp_password: default!(Option<&str>, "NULL"),
    ) -> String {
        let mailer = create_mailer(
            smtp_server,
            smtp_port,
            smtp_tls,
            smtp_username,
            smtp_password,
        )
        .expect("Failed to create mailer");

        let message = create_message(
            subject,
            body,
            html,
            from_address,
            recipients,
            ccs,
            bccs,
            false,
        )
        .expect("Failed to create message");

        let result = mailer.send(&message).expect("Failed to send email");
        if !result.is_positive() {
            panic!(
                "SMTP error {}: {}",
                result.code(),
                result.message().collect::<Vec<&str>>().join("; ")
            );
        }

        result.code().to_string()
    }

    #[cfg(any(test, feature = "pg_test"))]
    #[pg_schema]
    mod tests {
        use super::*;
        use std::collections::HashMap;

        fn extract_headers(message: &Message) -> HashMap<String, String> {
            let formatted_message = String::from_utf8(message.formatted()).unwrap();
            let lines = formatted_message.split("\r\n").collect::<Vec<&str>>();

            let header_lines: Vec<&str> = lines
                .iter()
                .take_while(|&&line| !line.is_empty())
                .cloned()
                .collect();

            let mut headers = HashMap::new();
            for line in header_lines {
                if let Some((key, value)) = line.split_once(": ") {
                    headers.insert(key.to_string(), value.to_string());
                }
            }

            headers
        }

        fn assert_header_value(message: &Message, header_name: &str, expected_value: &str) {
            let headers = extract_headers(message);
            assert_eq!(
                headers.get(header_name),
                Some(&expected_value.to_string()),
                "Header {} does not have the expected value.",
                header_name
            );
        }

        fn assert_header_missing(message: &Message, header_name: &str) {
            let headers = extract_headers(message);
            assert_eq!(
                headers.get(header_name),
                None,
                "Header {} should not be present.",
                header_name
            );
        }

        #[pg_test]
        fn test_send_email() {
            let result = send_email(
                "test subject",
                "test body",
                false,
                Some("from@example.com"),
                Some(vec![Some("to@example.com")]),
                None,
                None,
                Some("127.0.0.1"),
                Some(8025),
                Some(false),
                None,
                None,
            );

            assert_eq!(result, "250");
        }

        #[pg_test]
        fn test_send_email_with_smtp_config() {
            Spi::run("set smtp_client.from_address to 'from@example.com'")
                .expect("Failed to set smtp_client.from_address");
            Spi::run("set smtp_client.server to '127.0.0.1'")
                .expect("Failed to set smtp_client.server");
            Spi::run("set smtp_client.port to 8025").expect("Failed to set smtp_client.port");
            Spi::run("set smtp_client.tls to false").expect("Failed to set smtp_client.tls");

            let result = send_email(
                "test subject",
                "test body",
                false,
                None,
                Some(vec![Some("to@example.com")]),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            );

            assert_eq!(result, "250");
        }
        #[pg_test]
        #[should_panic]
        fn test_send_email_without_smtp_config() {
            send_email(
                "test subject",
                "test body",
                false,
                None,
                Some(vec![Some("to@example.com")]),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            );
        }

        #[pg_test]
        fn test_create_message_with_single_recipient() {
            let message = create_message(
                "Test Subject",
                "Test Body",
                false,
                Some("from@example.com"),
                Some(vec![Some("to@example.com")]),
                None,
                None,
                false,
            )
            .unwrap();

            assert_header_value(&message, "Subject", "Test Subject");
            assert_header_value(&message, "To", "to@example.com");
            assert_header_missing(&message, "Cc");
            assert_header_missing(&message, "Bcc");
            assert_header_value(&message, "From", "from@example.com");
        }

        #[pg_test]
        fn test_create_message_with_multiple_recipients() {
            let message = create_message(
                "Test Subject",
                "Test Body",
                false,
                Some("from@example.com"),
                Some(vec![Some("to1@example.com"), Some("to2@example.com")]),
                None,
                None,
                false,
            )
            .unwrap();

            assert_header_value(&message, "Subject", "Test Subject");
            assert_header_value(&message, "To", "to1@example.com, to2@example.com");
            assert_header_missing(&message, "Cc");
            assert_header_missing(&message, "Bcc");
            assert_header_value(&message, "From", "from@example.com");
        }

        #[pg_test]
        fn test_create_message_with_single_recipient_cc_and_bcc() {
            let message = create_message(
                "Test Subject",
                "Test Body",
                false,
                Some("from@example.com"),
                Some(vec![Some("to@example.com")]),
                Some(vec![Some("cc@example.com")]),
                Some(vec![Some("bcc@example.com")]),
                true,
            )
            .unwrap();

            assert_header_value(&message, "Subject", "Test Subject");
            assert_header_value(&message, "To", "to@example.com");
            assert_header_value(&message, "Cc", "cc@example.com");
            assert_header_value(&message, "Bcc", "bcc@example.com");
            assert_header_value(&message, "From", "from@example.com");
        }

        #[pg_test]
        fn test_create_message_with_multiple_recipients_ccs_and_bccs() {
            let message = create_message(
                "Test Subject",
                "Test Body",
                false,
                Some("from@example.com"),
                Some(vec![Some("to1@example.com"), Some("to2@example.com")]),
                Some(vec![Some("cc1@example.com"), Some("cc2@example.com")]),
                Some(vec![Some("bcc1@example.com"), Some("bcc2@example.com")]),
                true,
            )
            .unwrap();

            assert_header_value(&message, "Subject", "Test Subject");
            assert_header_value(&message, "To", "to1@example.com, to2@example.com");
            assert_header_value(&message, "Cc", "cc1@example.com, cc2@example.com");
            assert_header_value(&message, "Bcc", "bcc1@example.com, bcc2@example.com");
            assert_header_value(&message, "From", "from@example.com");
        }

        #[pg_test]
        fn test_create_message_from_smtp_from_config() {
            Spi::run("set smtp_client.from_address to 'from@example.com'")
                .expect("Failed to set smtp_client.from_address");

            let message = create_message(
                "Test Subject",
                "Test Body",
                false,
                None,
                Some(vec![Some("to@example.com")]),
                None,
                None,
                false,
            )
            .unwrap();

            assert_header_value(&message, "Subject", "Test Subject");
            assert_header_value(&message, "To", "to@example.com");
            assert_header_missing(&message, "Cc");
            assert_header_missing(&message, "Bcc");
            assert_header_value(&message, "From", "from@example.com");
        }

        #[pg_test]
        fn test_create_message_from_specified_address() {
            Spi::run("set smtp_client.from_address to 'from@example.com'")
                .expect("Failed to set smtp_client.from_address");

            let message = create_message(
                "Test Subject",
                "Test Body",
                false,
                Some("override@example.com"),
                Some(vec![Some("to@example.com")]),
                None,
                None,
                false,
            )
            .unwrap();

            assert_header_value(&message, "Subject", "Test Subject");
            assert_header_value(&message, "To", "to@example.com");
            assert_header_missing(&message, "Cc");
            assert_header_missing(&message, "Bcc");
            assert_header_value(&message, "From", "override@example.com");
        }

        #[pg_test]
        fn test_create_message_no_from() {
            let result = create_message(
                "Test Subject",
                "Test Body",
                false,
                None,
                Some(vec![Some("to@example.com")]),
                None,
                None,
                false,
            );

            assert!(result.is_err());
            assert_eq!(
                result.unwrap_err(),
                format!("From address not provided and no default configured")
            );
        }
    }
}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}
