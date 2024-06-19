# pg_smtp_client

A Postgres extension to send emails using SMTP.

## Usage

You use the `send_email` function to send an email. The function takes the following parameters:

| Parameter | Type | Description |
| --- | --- | --- |
| recipient | text | The email address of the recipient |
| subject | text | The subject of the email |
| body | text | The body of the email |
| is_html | boolean | Whether the body is HTML or plain text |
| from | text | The from email address |
| cc | text | The email address to CC |
| bcc | text | The email address to BCC |
| smtp_server | text | The SMTP server to use |
| smtp_port | integer | The port of the SMTP server |
| smtp_tls | boolean | Whether to use TLS |
| smtp_username | text | The username for the SMTP server |
| smtp_password | text | The password for the SMTP server |

### Environment Variable Configuration

You can configure default values for some of the parameters using environment variables. The following environment variables are supported:

| Environment Variable | Description |
| --- | --- |
| SMTP_SERVER | The default SMTP server to use |
| SMTP_PORT | The default port of the SMTP server |
| SMTP_TLS | Whether to use TLS |
| SMTP_USERNAME | The default username for the SMTP server |
| SMTP_PASSWORD | The default password for the SMTP server |
| SMTP_FROM | The default from email address |

### Examples

```sql
CREATE EXTENSION pg_smtp_client;
```

Send an email:
```sql
SELECT * FROM smtp.send_email('recipient@example.com', 'test subject', 'test body', false, null, null, 'from@example.com', 'smtp.example.com', 587, true, 'smtp_username', 'smtp_password');
```

Send an email using default values configured by environment variables:
```sql
SELECT * FROM smtp.send_email('recipient@example.com', 'test subject', 'test body');
```
