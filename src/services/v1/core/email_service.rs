use crate::infrastructure::mq::publish_email_message;
use crate::core::errors::AppError;

pub async fn send_verification_email(
    rabbitmq: &lapin::Connection,
    to: &str,
    name: &str,
    code: &str,
) -> Result<(), AppError> {
    // 1. Prepare email template
    let template_path = std::path::Path::new("templates/verification_email.html");
    let email_body = if template_path.exists() {
        let template_content = tokio::fs::read_to_string(template_path).await
            .unwrap_or_else(|_| "Your verification code is: {{code}}".to_string());
        template_content
            .replace("{{code}}", code)
            .replace("{{name}}", name)
    } else {
        format!("Welcome to Zent, {}! Your verification code is: {}", name, code)
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

pub async fn send_welcome_email(
    rabbitmq: &lapin::Connection,
    to: &str,
    name: &str,
) -> Result<(), AppError> {
    let email_payload = serde_json::json!({
        "to": to,
        "subject": "Welcome to Zent!",
        "body": format!("Welcome to Zent, {}! Your account has been successfully created.", name)
    });
    
    publish_email_message(rabbitmq, email_payload.to_string().as_bytes()).await
        .map_err(|e| {
            tracing::error!("Failed to enqueue welcome email task into RabbitMQ: {}", e);
            AppError::Internal(anyhow::anyhow!("Failed to send welcome email"))
        })?;

    Ok(())
}
