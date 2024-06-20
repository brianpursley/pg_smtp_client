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

### System-wide default configuration

You can configure default values for some of the parameters like this:

```
ALTER SYSTEM SET smtp_client.server TO 'smtp.example.com';
ALTER SYSTEM SET smtp_client.server TO 587;
ALTER SYSTEM SET smtp_client.tls TO true;
ALTER SYSTEM SET smtp_client.username TO 'MySmtpUsername';
ALTER SYSTEM SET smtp_client.password TO 'MySmtpPassword';
ALTER SYSTEM SET smtp_client.from TO 'from@example.com';
SELECT pg_reload_conf();
```

### Examples

```sql
CREATE EXTENSION pg_smtp_client;
```

Send an email:
```sql
SELECT * FROM smtp.send_email('recipient@example.com', 'test subject', 'test body', false, null, null, 'from@example.com', 'smtp.example.com', 587, true, 'smtp_username', 'smtp_password');
```

Send an email using configured default values:
```sql
SELECT * FROM smtp.send_email('recipient@example.com', 'test subject', 'test body');
```
