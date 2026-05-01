use std::collections::HashMap;
use std::sync::Arc;
use crate::infrastructure::mq::{RabbitMQClient, email::EmailProducer};
use crate::core::errors::AppError;

pub async fn send_verification_email(
    rabbitmq: &Arc<RabbitMQClient>,
    templates: &HashMap<String, String>,
    to: &str,
    name: &str,
    code: &str,
) -> Result<(), AppError> {
    // Escape variables for secure HTML rendering
    let escaped_name = v_htmlescape::escape(name).to_string();
    let escaped_code = v_htmlescape::escape(code).to_string();

    // 1. Prepare email template from cache
    let email_body = if let Some(template_content) = templates.get("verification_email.html") {
        template_content
            .replace("{{code}}", &escaped_code)
            .replace("{{name}}", &escaped_name)
    } else {
        tracing::warn!("Template 'verification_email.html' not found in cache! Using minimal HTML fallback.");
        format!(
            "<html><body><h2>Welcome to Zent, {}!</h2><p>Your verification code is: <strong style='color:#007bff; font-size:24px;'>{}</strong></p></body></html>", 
            escaped_name, escaped_code
        )
    };

    // 2. Deliver async email task to RabbitMQ
    let email_payload = serde_json::json!({
        "to": to,
        "subject": "Zent Account Verification",
        "body": email_body
    });
    
    let producer = EmailProducer::new(rabbitmq.clone());
    producer.publish(email_payload.to_string().as_bytes()).await
        .map_err(|e| {
            tracing::error!("Failed to enqueue verification email task into RabbitMQ: {}", e);
            AppError::Internal(anyhow::anyhow!("Failed to send verification email"))
        })?;

    Ok(())
}

pub async fn send_forgot_password_email(
    rabbitmq: &Arc<RabbitMQClient>,
    templates: &HashMap<String, String>,
    to: &str,
    name: &str,
    code: &str,
) -> Result<(), AppError> {
    let escaped_name = v_htmlescape::escape(name).to_string();
    let escaped_code = v_htmlescape::escape(code).to_string();

    let email_body = if let Some(template_content) = templates.get("forgot_password_email.html") {
        template_content
            .replace("{{code}}", &escaped_code)
            .replace("{{name}}", &escaped_name)
    } else {
        tracing::warn!("Template 'forgot_password_email.html' not found in cache! Using minimal HTML fallback.");
        format!(
            "<html><body><h2>Reset Your Password, {}</h2><p>Your password reset code is: <strong style='color:#dc3545; font-size:24px;'>{}</strong></p></body></html>", 
            escaped_name, escaped_code
        )
    };

    let email_payload = serde_json::json!({
        "to": to,
        "subject": "Zent Password Reset Request",
        "body": email_body
    });
    
    let producer = EmailProducer::new(rabbitmq.clone());
    producer.publish(email_payload.to_string().as_bytes()).await
        .map_err(|e| {
            tracing::error!("Failed to enqueue forgot password email task into RabbitMQ: {}", e);
            AppError::Internal(anyhow::anyhow!("Failed to send reset email"))
        })?;

    Ok(())
}

pub async fn send_welcome_email(
    rabbitmq: &Arc<RabbitMQClient>,
    _templates: &HashMap<String, String>,
    to: &str,
    name: &str,
) -> Result<(), AppError> {
    let escaped_name = v_htmlescape::escape(name).to_string();
    let email_payload = serde_json::json!({
        "to": to,
        "subject": "Welcome to Zent!",
        "body": format!("Welcome to Zent, {}! Your account has been successfully created.", escaped_name)
    });
    
    let producer = EmailProducer::new(rabbitmq.clone());
    producer.publish(email_payload.to_string().as_bytes()).await
        .map_err(|e| {
            tracing::error!("Failed to enqueue welcome email task into RabbitMQ: {}", e);
            AppError::Internal(anyhow::anyhow!("Failed to send welcome email"))
        })?;

    Ok(())
}
