use crate::infrastructure::mq::publish_email_message;
use crate::core::errors::AppError;

pub async fn send_verification_email(
    rabbitmq: &lapin::Connection,
    to: &str,
    name: &str,
    code: &str,
) -> Result<(), AppError> {
    // Escape variables for secure HTML rendering
    let escaped_name = v_htmlescape::escape(name).to_string();
    let escaped_code = v_htmlescape::escape(code).to_string();

    // 1. Prepare email template
    let template_path = std::path::Path::new("templates/verification_email.html");
    let email_body = if template_path.exists() {
        let template_content = tokio::fs::read_to_string(template_path).await
            .unwrap_or_else(|_| "Your verification code is: {{code}}".to_string());
        template_content
            .replace("{{code}}", &escaped_code)
            .replace("{{name}}", &escaped_name)
    } else {
        format!("Welcome to Zent, {}! Your verification code is: {}", escaped_name, escaped_code)
    };

    // 2. Deliver async email task to RabbitMQ
    let email_payload = serde_json::json!({
        "to": to,
        "subject": "Zent Account Verification",
        "body": email_body
    });
    
    publish_email_message(rabbitmq, email_payload.to_string().as_bytes()).await
        .map_err(|e| {
            tracing::error!("Failed to enqueue verification email task into RabbitMQ: {}", e);
            AppError::Internal(anyhow::anyhow!("Failed to send verification email"))
        })?;

    Ok(())
}

pub async fn send_forgot_password_email(
    rabbitmq: &lapin::Connection,
    to: &str,
    name: &str,
    code: &str,
) -> Result<(), AppError> {
    let escaped_name = v_htmlescape::escape(name).to_string();
    let escaped_code = v_htmlescape::escape(code).to_string();

    let template_path = std::path::Path::new("templates/forgot_password_email.html");
    let email_body = if template_path.exists() {
        let template_content = tokio::fs::read_to_string(template_path).await
            .unwrap_or_else(|_| "Your password reset code is: {{code}}".to_string());
        template_content
            .replace("{{code}}", &escaped_code)
            .replace("{{name}}", &escaped_name)
    } else {
        format!("Hello {}, Your password reset code is: {}", escaped_name, escaped_code)
    };

    let email_payload = serde_json::json!({
        "to": to,
        "subject": "Zent Password Reset Request",
        "body": email_body
    });
    
    publish_email_message(rabbitmq, email_payload.to_string().as_bytes()).await
        .map_err(|e| {
            tracing::error!("Failed to enqueue forgot password email task into RabbitMQ: {}", e);
            AppError::Internal(anyhow::anyhow!("Failed to send reset email"))
        })?;

    Ok(())
}

pub async fn send_welcome_email(
    rabbitmq: &lapin::Connection,
    to: &str,
    name: &str,
) -> Result<(), AppError> {
    let escaped_name = v_htmlescape::escape(name).to_string();
    let email_payload = serde_json::json!({
        "to": to,
        "subject": "Welcome to Zent!",
        "body": format!("Welcome to Zent, {}! Your account has been successfully created.", escaped_name)
    });
    
    publish_email_message(rabbitmq, email_payload.to_string().as_bytes()).await
        .map_err(|e| {
            tracing::error!("Failed to enqueue welcome email task into RabbitMQ: {}", e);
            AppError::Internal(anyhow::anyhow!("Failed to send welcome email"))
        })?;

    Ok(())
}
