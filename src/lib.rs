use pgrx::prelude::*;

pg_module_magic!();

#[pg_schema]
mod smtp_client {
    use super::*;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::transport::smtp::client::{Tls, TlsParameters};
    use lettre::{Message, SmtpTransport, Transport};
    use std::env;

    const DEFAULT_SMTP_PORT: u16 = 587;
    const DEFAULT_SMTP_TLS: bool = true;

    const SMTP_SERVER_ENV_VAR: &str = "SMTP_SERVER";
    const SMTP_PORT_ENV_VAR: &str = "SMTP_PORT";
    const SMTP_TLS_ENV_VAR: &str = "SMTP_TLS";
    const SMTP_USERNAME_ENV_VAR: &str = "SMTP_USERNAME";
    const SMTP_PASSWORD_ENV_VAR: &str = "SMTP_PASSWORD";
    const SMTP_FROM_ENV_VAR: &str = "SMTP_FROM";

    #[derive(Debug, PartialEq)]
    struct SmtpConfig {
        server_name: String,
        port: u16,
        tls: bool,
        username: String,
        password: String,
    }

    fn create_smtp_config(
        smtp_server: Option<&str>,
        smtp_port: Option<u16>,
        smtp_tls: Option<bool>,
        smtp_username: Option<&str>,
        smtp_password: Option<&str>,
    ) -> Result<SmtpConfig, String> {
        Ok(SmtpConfig {
            server_name: smtp_server
                .map_or_else(
                    || match env::var(SMTP_SERVER_ENV_VAR) {
                        Ok(server) => Ok(server),
                        Err(_) => Err(format!("SMTP server not provided and {SMTP_SERVER_ENV_VAR} environment variable not set")),
                    },
                    |s| Ok(s.to_string()),
                )?,
            port: smtp_port
                .map_or_else(
                    || match env::var(SMTP_PORT_ENV_VAR) {
                        Ok(port) => port.parse::<u16>().map_err(|e| format!("SMTP port not provided and {SMTP_PORT_ENV_VAR} environment variable is invalid ({e})")),
                        Err(_) => Ok(DEFAULT_SMTP_PORT),
                    },
                    |s| Ok(s),
                )?,
            tls: smtp_tls
                .map_or_else(
                    || match env::var(SMTP_TLS_ENV_VAR) {
                        Ok(tls) => tls.parse::<bool>().map_err(|e| format!("SMTP TLS not provided and {SMTP_TLS_ENV_VAR} environment variable is invalid ({e})")),
                        Err(_) => Ok(DEFAULT_SMTP_TLS),
                    },
                    |s| Ok(s),
                )?,
            username: smtp_username
                .map_or_else(
                    || env::var(SMTP_USERNAME_ENV_VAR).unwrap_or_default(),
                    |s| s.to_string(),
                ),
            password: smtp_password
                .map_or_else(
                    || env::var(SMTP_PASSWORD_ENV_VAR).unwrap_or_default(),
                    |s| s.to_string(),
                ),
        })
    }

    fn create_mailer(smtp_config: SmtpConfig) -> Result<SmtpTransport, String> {
        let mut mailer = SmtpTransport::relay(&smtp_config.server_name)
            .map_err(|e| format!("Failed to create SMTP relay: {:?}", e))?
            .port(smtp_config.port);

        if smtp_config.tls {
            let tls_parameters = TlsParameters::new(smtp_config.server_name)
                .map_err(|e| format!("Failed to create TLS parameters: {:?}", e))?;
            mailer = mailer.tls(Tls::Wrapper(tls_parameters));
        } else {
            mailer = mailer.tls(Tls::None);
        }

        if !smtp_config.username.is_empty() || !smtp_config.password.is_empty() {
            mailer =
                mailer.credentials(Credentials::new(smtp_config.username, smtp_config.password));
        }

        Ok(mailer.build())
    }

    fn create_message(
        recipient: &str,
        subject: &str,
        body: &str,
        is_html: bool,
        from: Option<&str>,
        cc: Option<&str>,
        bcc: Option<&str>,
        keep_bcc: bool,
    ) -> Result<Message, String> {
        let mut email = Message::builder().subject(subject);

        if is_html {
            email = email.header(lettre::message::header::ContentType::TEXT_HTML);
        }

        for recipient_address in recipient.split(',') {
            email = email.to(match recipient_address.trim().parse() {
                Ok(address) => address,
                Err(e) => return Err(format!("Invalid recipient address: {}", e)),
            });
        }

        if let Some(cc_list) = cc {
            if !cc_list.trim().is_empty() {
                for cc_address in cc_list.split(',') {
                    email = email.cc(match cc_address.trim().parse() {
                        Ok(address) => address,
                        Err(e) => return Err(format!("Invalid cc address: {}", e)),
                    });
                }
            }
        }

        if let Some(bcc_list) = bcc {
            if !bcc_list.trim().is_empty() {
                if keep_bcc {
                    email = email.keep_bcc();
                }
                for bcc_address in bcc_list.split(',') {
                    email = email.bcc(match bcc_address.trim().parse() {
                        Ok(address) => address,
                        Err(e) => return Err(format!("Invalid bcc address: {}", e)),
                    });
                }
            }
        }

        let from_address = from.map_or_else(
            || match env::var(SMTP_FROM_ENV_VAR) {
                Ok(from) => Ok(from),
                Err(_) => match env::var(SMTP_USERNAME_ENV_VAR) {
                    Ok(username) => Ok(username),
                    Err(_) => Err(format!("From address not provided and {SMTP_FROM_ENV_VAR} or {SMTP_USERNAME_ENV_VAR} environment variable not set")),
                }
            },
            |s| Ok(s.to_string())
        )?;
        email = email.from(match from_address.trim().parse() {
            Ok(address) => address,
            Err(e) => return Err(format!("Invalid from address: {}", e)),
        });

        email.body(body.to_string()).map_err(|e| e.to_string())
    }

    #[pg_extern]
    fn send_email(
        recipient: &str,
        subject: &str,
        body: &str,
        is_html: default!(bool, "false"),
        from: default!(Option<&str>, "NULL"),
        cc: default!(Option<&str>, "NULL"),
        bcc: default!(Option<&str>, "NULL"),
        smtp_server: default!(Option<&str>, "NULL"),
        smtp_port: default!(Option<i32>, "NULL"),
        smtp_tls: default!(Option<bool>, "NULL"),
        smtp_username: default!(Option<&str>, "NULL"),
        smtp_password: default!(Option<&str>, "NULL"),
    ) -> String {
        let smtp_config = create_smtp_config(
            smtp_server,
            smtp_port.map(|value| value as u16),
            smtp_tls,
            smtp_username,
            smtp_password,
        )
        .expect("Could not create SMTP config");

        let mailer = create_mailer(smtp_config).expect("Could not create mailer");

        let message = create_message(recipient, subject, body, is_html, from, cc, bcc, false)
            .expect("Could not create message");

        let result = mailer.send(&message).expect("Could not send email");

        result.code().to_string()
    }

    #[cfg(any(test, feature = "pg_test"))]
    #[pg_schema]
    mod tests {
        use super::*;
        use serial_test::serial;
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
        #[serial]
        fn test_send_email() {
            env::remove_var(SMTP_USERNAME_ENV_VAR);
            env::remove_var(SMTP_PASSWORD_ENV_VAR);

            let result = send_email(
                "to@example.com",
                "test subject",
                "test body",
                false,
                Some("from@example.com"),
                None,
                None,
                Some("127.0.0.1"),
                Some(2525),
                Some(false),
                None,
                None,
            );

            assert_eq!(result, "Email sent successfully");
        }

        #[pg_test]
        #[should_panic]
        fn test_send_email_without_smtp_config() {
            env::remove_var(SMTP_SERVER_ENV_VAR);
            env::remove_var(SMTP_PORT_ENV_VAR);
            env::remove_var(SMTP_TLS_ENV_VAR);
            env::remove_var(SMTP_USERNAME_ENV_VAR);
            env::remove_var(SMTP_PASSWORD_ENV_VAR);

            send_email(
                "to@example.com",
                "test subject",
                "test body",
                false,
                None,
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
        fn test_create_smtp_config_with_provided_parameters() {
            let smtp_server = "smtp.example.com";
            let smtp_port = 587;
            let smtp_tls = true;
            let smtp_username = "smtp-username";
            let smtp_password = "smtp-password";

            let config = create_smtp_config(
                Some(smtp_server),
                Some(smtp_port),
                Some(smtp_tls),
                Some(smtp_username),
                Some(smtp_password),
            )
            .unwrap();

            // Verify the mailer is correctly configured (though we can't directly inspect mailer properties in lettre)
            assert_eq!(config.server_name, smtp_server.to_string());
            assert_eq!(config.port, smtp_port);
            assert_eq!(config.tls, smtp_tls);
            assert_eq!(config.username, smtp_username.to_string());
            assert_eq!(config.password, smtp_password.to_string());
        }

        #[pg_test]
        fn test_create_smtp_config_with_env_defaults() {
            env::set_var(SMTP_SERVER_ENV_VAR, "smtp.example.com");
            env::set_var(SMTP_PORT_ENV_VAR, "8587");
            env::set_var(SMTP_TLS_ENV_VAR, "false");
            env::set_var(SMTP_USERNAME_ENV_VAR, "smtp-username");
            env::set_var(SMTP_PASSWORD_ENV_VAR, "smtp-password");

            let config = create_smtp_config(None, None, None, None, None).unwrap();

            // Verify the mailer is correctly configured (though we can't directly inspect mailer properties in lettre)
            assert_eq!(config.server_name, "smtp.example.com".to_string());
            assert_eq!(config.port, 8587);
            assert_eq!(config.tls, false);
            assert_eq!(config.username, "smtp-username".to_string());
            assert_eq!(config.password, "smtp-password".to_string());

            env::remove_var(SMTP_SERVER_ENV_VAR);
            env::remove_var(SMTP_PORT_ENV_VAR);
            env::remove_var(SMTP_TLS_ENV_VAR);
            env::remove_var(SMTP_USERNAME_ENV_VAR);
            env::remove_var(SMTP_PASSWORD_ENV_VAR);
        }

        #[pg_test]
        fn test_create_smtp_config_missing_configuration() {
            env::remove_var(SMTP_SERVER_ENV_VAR);
            let result = create_smtp_config(None, None, None, None, None);
            assert!(result.is_err());
            assert_eq!(
                result.unwrap_err(),
                format!(
                "SMTP server not provided and {SMTP_SERVER_ENV_VAR} environment variable not set"
            )
            );
        }

        #[pg_test]
        fn test_create_message_with_single_recipient() {
            let message = create_message(
                "to@example.com",
                "Test Subject",
                "Test Body",
                false,
                Some("from@example.com"),
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
                "to1@example.com,to2@example.com",
                "Test Subject",
                "Test Body",
                false,
                Some("from@example.com"),
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
                "to@example.com",
                "Test Subject",
                "Test Body",
                false,
                Some("from@example.com"),
                Some("cc@example.com"),
                Some("bcc@example.com"),
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
                "to1@example.com,to2@example.com",
                "Test Subject",
                "Test Body",
                false,
                Some("from@example.com"),
                Some("cc1@example.com,cc2@example.com"),
                Some("bcc1@example.com,bcc2@example.com"),
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
        fn test_create_message_without_keep_bcc() {
            let message = create_message(
                "to@example.com",
                "Test Subject",
                "Test Body",
                false,
                Some("from@example.com"),
                Some("cc@example.com"),
                Some("bcc@example.com"),
                false,
            )
            .unwrap();

            assert_header_value(&message, "Subject", "Test Subject");
            assert_header_value(&message, "To", "to@example.com");
            assert_header_value(&message, "Cc", "cc@example.com");
            assert_header_missing(&message, "Bcc");
            assert_header_value(&message, "From", "from@example.com");
        }

        #[pg_test]
        fn test_create_message_from_smtp_from_env() {
            env::set_var(SMTP_USERNAME_ENV_VAR, "username@example.com");
            env::set_var(SMTP_FROM_ENV_VAR, "from@example.com");

            let message = create_message(
                "to@example.com",
                "Test Subject",
                "Test Body",
                false,
                None,
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

            env::remove_var(SMTP_USERNAME_ENV_VAR);
            env::remove_var(SMTP_FROM_ENV_VAR);
        }

        #[pg_test]
        fn test_create_message_from_smtp_username_env() {
            env::set_var(SMTP_USERNAME_ENV_VAR, "username@example.com");
            env::remove_var(SMTP_FROM_ENV_VAR);

            let message = create_message(
                "to@example.com",
                "Test Subject",
                "Test Body",
                false,
                None,
                None,
                None,
                false,
            )
            .unwrap();

            assert_header_value(&message, "Subject", "Test Subject");
            assert_header_value(&message, "To", "to@example.com");
            assert_header_missing(&message, "Cc");
            assert_header_missing(&message, "Bcc");
            assert_header_value(&message, "From", "username@example.com");

            env::remove_var(SMTP_USERNAME_ENV_VAR);
        }

        #[pg_test]
        fn test_create_message_from_specified_address() {
            env::set_var(SMTP_USERNAME_ENV_VAR, "username@example.com");
            env::set_var(SMTP_FROM_ENV_VAR, "defaultfrom@example.com");

            let message = create_message(
                "to@example.com",
                "Test Subject",
                "Test Body",
                false,
                Some("from@example.com"),
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

            env::remove_var(SMTP_USERNAME_ENV_VAR);
            env::remove_var(SMTP_FROM_ENV_VAR);
        }

        #[pg_test]
        fn test_create_message_no_from() {
            env::remove_var(SMTP_USERNAME_ENV_VAR);
            env::remove_var(SMTP_FROM_ENV_VAR);

            let result = create_message(
                "to@example.com",
                "Test Subject",
                "Test Body",
                false,
                None,
                None,
                None,
                false,
            );

            assert!(result.is_err());
            assert_eq!(
            result.unwrap_err(),
            format!(
                "From address not provided and {SMTP_FROM_ENV_VAR} or {SMTP_USERNAME_ENV_VAR} environment variable not set"
            )
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
