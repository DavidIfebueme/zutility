use anyhow::Result;
use secrecy::{ExposeSecret, SecretString};
use serde_json::json;

#[derive(Clone)]
pub struct EmailClient {
    api_key: String,
    sender_email: String,
    sender_name: String,
    http: reqwest::Client,
}

impl EmailClient {
    pub fn new(api_key: &SecretString, sender_email: &str, sender_name: &str) -> Self {
        Self {
            api_key: api_key.expose_secret().to_owned(),
            sender_email: sender_email.to_owned(),
            sender_name: sender_name.to_owned(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn send_verification_email(&self, to: &str, verification_link: &str) -> Result<()> {
        let html = format!(
            r#"<html><body style="font-family:system-ui,sans-serif;max-width:480px;margin:0 auto;padding:24px;background:#0a0a0a;color:#fafafa">
  <div style="border:1px solid #262626;border-radius:12px;padding:32px;background:#141414">
    <h1 style="font-size:24px;margin:0 0 8px;color:#fafafa">Verify your email</h1>
    <p style="color:#a3a3a3;margin:0 0 24px">Welcome to <span style="color:#f4b731">z</span>utility. Click the button below to verify your email address.</p>
    <a href="{verification_link}" style="display:inline-block;background:#f4b731;color:#0a0a0a;padding:12px 32px;border-radius:8px;text-decoration:none;font-weight:600;font-size:16px">Verify Email</a>
    <p style="color:#a3a3a3;font-size:14px;margin:24px 0 0">If the button doesn't work, copy and paste this link into your browser:</p>
    <p style="color:#f4b731;font-size:13px;word-break:break-all;margin:8px 0 0">{verification_link}</p>
    <p style="color:#737373;font-size:13px;margin:24px 0 0">If you didn&apos;t create an account, you can ignore this email.</p>
    <hr style="border:none;border-top:1px solid #262626;margin:24px 0" />
    <p style="color:#525252;font-size:12px;margin:0">This link expires in 24 hours.</p>
  </div>
</body></html>"#
        );

        self.send(to, "Verify your email — zutility", &html).await
    }

    pub async fn send_password_reset_email(&self, to: &str, reset_link: &str) -> Result<()> {
        let html = format!(
            r#"<html><body style="font-family:system-ui,sans-serif;max-width:480px;margin:0 auto;padding:24px;background:#0a0a0a;color:#fafafa">
  <div style="border:1px solid #262626;border-radius:12px;padding:32px;background:#141414">
    <h1 style="font-size:24px;margin:0 0 8px;color:#fafafa">Reset your password</h1>
    <p style="color:#a3a3a3;margin:0 0 24px">Click the button below to set a new password for your <span style="color:#f4b731">z</span>utility account.</p>
    <a href="{reset_link}" style="display:inline-block;background:#f4b731;color:#0a0a0a;padding:12px 32px;border-radius:8px;text-decoration:none;font-weight:600;font-size:16px">Reset Password</a>
    <p style="color:#a3a3a3;font-size:14px;margin:24px 0 0">If the button doesn't work, copy and paste this link into your browser:</p>
    <p style="color:#f4b731;font-size:13px;word-break:break-all;margin:8px 0 0">{reset_link}</p>
    <p style="color:#737373;font-size:13px;margin:24px 0 0">If you didn&apos;t request a password reset, you can ignore this email.</p>
    <hr style="border:none;border-top:1px solid #262626;margin:24px 0" />
    <p style="color:#525252;font-size:12px;margin:0">This link expires in 1 hour.</p>
  </div>
</body></html>"#
        );

        self.send(to, "Reset your password — zutility", &html).await
    }

    pub async fn send_waitlist_verification_email(&self, to: &str, verification_link: &str) -> Result<()> {
        let html = format!(
            r#"<html><body style="font-family:system-ui,sans-serif;max-width:480px;margin:0 auto;padding:24px;background:#0a0a0a;color:#fafafa">
  <div style="border:1px solid #262626;border-radius:12px;padding:32px;background:#141414">
    <h1 style="font-size:24px;margin:0 0 8px;color:#fafafa">You're on the list!</h1>
    <p style="color:#a3a3a3;margin:0 0 24px">Thanks for joining the <span style="color:#f4b731">z</span>utility waitlist. Confirm your email to secure your spot.</p>
    <a href="{verification_link}" style="display:inline-block;background:#f4b731;color:#0a0a0a;padding:12px 32px;border-radius:8px;text-decoration:none;font-weight:600;font-size:16px">Confirm Email</a>
    <p style="color:#a3a3a3;font-size:14px;margin:24px 0 0">If the button doesn't work, copy and paste this link into your browser:</p>
    <p style="color:#f4b731;font-size:13px;word-break:break-all;margin:8px 0 0">{verification_link}</p>
    <p style="color:#737373;font-size:13px;margin:24px 0 0">If you didn't sign up for the waitlist, you can ignore this email.</p>
    <hr style="border:none;border-top:1px solid #262626;margin:24px 0" />
    <p style="color:#525252;font-size:12px;margin:0">This link expires in 24 hours.</p>
  </div>
</body></html>"#
        );

        self.send(to, "Confirm your email — zutility waitlist", &html).await
    }

    pub async fn send_support_email(&self, from_email: &str, from_name: &str, subject: &str, message: &str) -> Result<()> {
        let html = format!(
            r#"<html><body style="font-family:system-ui,sans-serif;max-width:560px;margin:0 auto;padding:24px;background:#0a0a0a;color:#fafafa">
  <div style="border:1px solid #262626;border-radius:12px;padding:32px;background:#141414">
    <h1 style="font-size:20px;margin:0 0 16px;color:#fafafa">New Support Request</h1>
    <table style="width:100%;border-collapse:collapse;font-size:14px">
      <tr><td style="padding:4px 0;color:#a3a3a3;width:80px">From</td><td style="padding:4px 0;color:#fafafa">{from_name} ({from_email})</td></tr>
      <tr><td style="padding:4px 0;color:#a3a3a3">Subject</td><td style="padding:4px 0;color:#f4b731">{subject}</td></tr>
    </table>
    <hr style="border:none;border-top:1px solid #262626;margin:16px 0" />
    <div style="color:#d4d4d4;font-size:14px;line-height:1.6;white-space:pre-wrap">{message}</div>
    <hr style="border:none;border-top:1px solid #262626;margin:16px 0" />
    <p style="color:#525252;font-size:12px;margin:0">Reply to {from_email} or respond via the zutility admin.</p>
  </div>
</body></html>"#
        );

        self.send(&self.sender_email, &format!("[Support] {subject}"), &html).await
    }

    async fn send(&self, to: &str, subject: &str, html_content: &str) -> Result<()> {
        let payload = json!({
            "sender": {
                "name": self.sender_name,
                "email": self.sender_email,
            },
            "to": [{ "email": to }],
            "subject": subject,
            "htmlContent": html_content,
            "disableTrackClicks": true,
        });

        let response = self
            .http
            .post("https://api.brevo.com/v3/smtp/email")
            .header("api-key", &self.api_key)
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            tracing::warn!(status = %status, body = %body, "brevo email send failed");
            anyhow::bail!("brevo email send failed with status {}", status);
        }

        Ok(())
    }
}
