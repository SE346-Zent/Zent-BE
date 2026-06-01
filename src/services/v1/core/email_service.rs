use std::collections::HashMap;
use std::sync::Arc;
use crate::core::errors::AppError;
use lapin::Connection;
use crate::services::v1::core::helpers::mq::publish_email_task;

/// Dispatch a verification email containing a security code for account activation.

pub async fn send_verification_email(
    rabbitmq_connection: &Arc<Connection>,
    templates: &HashMap<String, String>,
    recipient_email: &str,
    recipient_name: &str,
    security_code: &str,
) -> Result<(), AppError> {
    // Escape variables for secure HTML rendering
    let escaped_name = v_htmlescape::escape(recipient_name).to_string();
    let escaped_code = v_htmlescape::escape(security_code).to_string();

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
        "to": recipient_email,
        "subject": "Zent Account Verification",
        "body": email_body
    });
    
    publish_email_task(rabbitmq_connection, email_payload, "verification email").await
}

/// Dispatch a password reset email containing a temporary recovery code.

pub async fn send_forgot_password_email(
    rabbitmq_connection: &Arc<Connection>,
    templates: &HashMap<String, String>,
    recipient_email: &str,
    recipient_name: &str,
    security_code: &str,
) -> Result<(), AppError> {
    let escaped_name = v_htmlescape::escape(recipient_name).to_string();
    let escaped_code = v_htmlescape::escape(security_code).to_string();

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
        "to": recipient_email,
        "subject": "Zent Password Reset Request",
        "body": email_body
    });
    
    publish_email_task(rabbitmq_connection, email_payload, "reset email").await
}

/// Dispatch a welcome email to newly registered users upon successful verification.

pub async fn send_welcome_email(
    rabbitmq_connection: &Arc<Connection>,
    _templates: &HashMap<String, String>,
    recipient_email: &str,
    recipient_name: &str,
) -> Result<(), AppError> {
    let escaped_name = v_htmlescape::escape(recipient_name).to_string();
    let email_payload = serde_json::json!({
        "to": recipient_email,
        "subject": "Welcome to Zent!",
        "body": format!("Welcome to Zent, {}! Your account has been successfully created.", escaped_name)
    });
    
    publish_email_task(rabbitmq_connection, email_payload, "welcome email").await
}

/// Dispatch a welcome email containing credentials for newly created user.
pub async fn send_create_user_email(
    rabbitmq_connection: &Arc<Connection>,
    templates: &HashMap<String, String>,
    recipient_email: &str,
    recipient_name: &str,
    plain_password: &str,
) -> Result<(), AppError> {
    let escaped_name = v_htmlescape::escape(recipient_name).to_string();
    let escaped_email = v_htmlescape::escape(recipient_email).to_string();
    let escaped_password = v_htmlescape::escape(plain_password).to_string();

    let email_body = if let Some(template_content) = templates.get("create_user_email.html") {
        template_content
            .replace("{{name}}", &escaped_name)
            .replace("{{email}}", &escaped_email)
            .replace("{{password}}", &escaped_password)
    } else {
        tracing::warn!("Template 'create_user_email.html' not found in cache! Using minimal HTML fallback.");
        format!(
            "<html><body><h2>Welcome to Zent, {}!</h2><p>Your account has been created.</p><p>Username: {}</p><p>Password: {}</p><p>Please log in and change your password.</p></body></html>", 
            escaped_name, escaped_email, escaped_password
        )
    };

    let email_payload = serde_json::json!({
        "to": recipient_email,
        "subject": "Your Zent Account",
        "body": email_body
    });
    
    publish_email_task(rabbitmq_connection, email_payload, "create user email").await
}

/// Notify a customer via email that their work order request has been successfully received.

pub async fn send_work_order_created_email(
    rabbitmq_connection: &Arc<Connection>,
    templates: &HashMap<String, String>,
    recipient_email: &str,
    recipient_name: &str,
    work_order_number: &str,
    service_type: &str,
    appointment: &str,
    address: &str,
) -> Result<(), AppError> {
    let escaped_name = v_htmlescape::escape(recipient_name).to_string();
    let escaped_wo_number = v_htmlescape::escape(work_order_number).to_string();
    let escaped_service = v_htmlescape::escape(service_type).to_string();
    let escaped_appointment = v_htmlescape::escape(appointment).to_string();
    let escaped_address = v_htmlescape::escape(address).to_string();

    let email_body = if let Some(template_content) = templates.get("work_order_created_email.html") {
        template_content
            .replace("{{name}}", &escaped_name)
            .replace("{{work_order_number}}", &escaped_wo_number)
            .replace("{{service_type}}", &escaped_service)
            .replace("{{appointment}}", &escaped_appointment)
            .replace("{{address}}", &escaped_address)
    } else {
        tracing::warn!("Template 'work_order_created_email.html' not found in cache! Using minimal HTML fallback.");
        format!(
            "<html><body><h2>Work Order Created, {}!</h2><p>Your work order number is: <strong>{}</strong></p><p>Service: {}</p><p>Appointment: {}</p><p>Address: {}</p></body></html>",
            escaped_name, escaped_wo_number, escaped_service, escaped_appointment, escaped_address
        )
    };

    let email_payload = serde_json::json!({
        "to": recipient_email,
        "subject": format!("Work Order Created: {}", work_order_number),
        "body": email_body
    });
    
    publish_email_task(rabbitmq_connection, email_payload, "work order creation email").await
}

/// Inform a customer that a technician's refusal was denied and the order is being reassigned.

pub async fn send_work_order_refusal_denied_email(
    rabbitmq_connection: &Arc<Connection>,
    templates: &HashMap<String, String>,
    recipient_email: &str,
    recipient_name: &str,
    work_order_number: &str,
) -> Result<(), AppError> {
    let escaped_name = v_htmlescape::escape(recipient_name).to_string();
    let escaped_wo_number = v_htmlescape::escape(work_order_number).to_string();

    let email_body = if let Some(template_content) = templates.get("work_order_refusal_denied_email.html") {
        template_content
            .replace("{{name}}", &escaped_name)
            .replace("{{work_order_number}}", &escaped_wo_number)
    } else {
        tracing::warn!("Template 'work_order_refusal_denied_email.html' not found in cache! Using minimal HTML fallback.");
        format!(
            "<html><body><h2>Work Order Update, {}!</h2><p>We regret to inform you that the technician's refusal for work order <strong>{}</strong> has been declined. Your work order remains active and will be reassigned.</p><p>We apologize for any inconvenience.</p></body></html>",
            escaped_name, escaped_wo_number
        )
    };

    let email_payload = serde_json::json!({
        "to": recipient_email,
        "subject": format!("Work Order Update: {}", work_order_number),
        "body": email_body
    });

    publish_email_task(rabbitmq_connection, email_payload, "work order refusal denied email").await
}

/// Notify a customer that a specific technician has been assigned to their work order.

pub async fn send_work_order_assigned_email(
    rabbitmq_connection: &Arc<Connection>,
    templates: &HashMap<String, String>,
    recipient_email: &str,
    recipient_name: &str,
    work_order_number: &str,
    technician_name: &str,
    appointment: &str,
) -> Result<(), AppError> {
    let escaped_name = v_htmlescape::escape(recipient_name).to_string();
    let escaped_wo_number = v_htmlescape::escape(work_order_number).to_string();
    let escaped_tech_name = v_htmlescape::escape(technician_name).to_string();
    let escaped_appointment = v_htmlescape::escape(appointment).to_string();

    let email_body = if let Some(template_content) = templates.get("work_order_assigned_email.html") {
        template_content
            .replace("{{name}}", &escaped_name)
            .replace("{{work_order_number}}", &escaped_wo_number)
            .replace("{{technician_name}}", &escaped_tech_name)
            .replace("{{appointment}}", &escaped_appointment)
    } else {
        tracing::warn!("Template 'work_order_assigned_email.html' not found in cache! Using minimal HTML fallback.");
        format!(
            "<html><body><h2>Work Order Assigned, {}!</h2><p>Your work order number is: <strong>{}</strong></p><p>Technician: {}</p><p>Appointment: {}</p></body></html>",
            escaped_name, escaped_wo_number, escaped_tech_name, escaped_appointment
        )
    };

    let email_payload = serde_json::json!({
        "to": recipient_email,
        "subject": format!("Work Order Assigned: {}", work_order_number),
        "body": email_body
    });
    
    publish_email_task(rabbitmq_connection, email_payload, "work order assigned email").await
}

pub async fn send_work_order_reassigned_email(
    rabbitmq: &Arc<Connection>,
    templates: &HashMap<String, String>,
    to: &str,
    name: &str,
    work_order_number: &str,
    technician_name: &str,
    appointment: &str,
) -> Result<(), AppError> {
    let escaped_name = v_htmlescape::escape(name).to_string();
    let escaped_wo_number = v_htmlescape::escape(work_order_number).to_string();
    let escaped_tech_name = v_htmlescape::escape(technician_name).to_string();
    let escaped_appointment = v_htmlescape::escape(appointment).to_string();

    let email_body = if let Some(template_content) = templates.get("work_order_reassigned_email.html") {
        template_content
            .replace("{{name}}", &escaped_name)
            .replace("{{work_order_number}}", &escaped_wo_number)
            .replace("{{technician_name}}", &escaped_tech_name)
            .replace("{{appointment}}", &escaped_appointment)
    } else {
        tracing::warn!("Template 'work_order_reassigned_email.html' not found in cache! Using minimal HTML fallback.");
        format!(
            "<html><body><h2>Work Order Reassigned, {}!</h2><p>Your work order <strong>{}</strong> has been reassigned to technician <strong>{}</strong>.</p><p>Appointment: {}</p></body></html>",
            escaped_name, escaped_wo_number, escaped_tech_name, escaped_appointment
        )
    };

    let email_payload = serde_json::json!({
        "to": to,
        "subject": format!("Work Order Reassigned: {}", work_order_number),
        "body": email_body
    });
    
    publish_email_task(rabbitmq, email_payload, "work order reassigned email").await
}

/// Dispatch a device registration confirmation email with registration details and warranty status.

pub async fn send_device_registration_email(
    rabbitmq_connection: &Arc<Connection>,
    templates: &HashMap<String, String>,
    recipient_email: &str,
    recipient_name: &str,
    product_name: &str,
    serial_number: &str,
    country: &str,
    province: &str,
    address: &str,
    warranty_status: &str,
    registration_date: &str,
) -> Result<(), AppError> {
    let escaped_name = v_htmlescape::escape(recipient_name).to_string();
    let escaped_product = v_htmlescape::escape(product_name).to_string();
    let escaped_serial = v_htmlescape::escape(serial_number).to_string();
    let escaped_country = v_htmlescape::escape(country).to_string();
    let escaped_province = v_htmlescape::escape(province).to_string();
    let escaped_address = v_htmlescape::escape(address).to_string();
    let escaped_warranty = v_htmlescape::escape(warranty_status).to_string();
    let escaped_date = v_htmlescape::escape(registration_date).to_string();

    let email_body = if let Some(template_content) = templates.get("device_registration_email.html") {
        template_content
            .replace("{{name}}", &escaped_name)
            .replace("{{product_name}}", &escaped_product)
            .replace("{{serial_number}}", &escaped_serial)
            .replace("{{country}}", &escaped_country)
            .replace("{{province}}", &escaped_province)
            .replace("{{address}}", &escaped_address)
            .replace("{{warranty_status}}", &escaped_warranty)
            .replace("{{registration_date}}", &escaped_date)
    } else {
        tracing::warn!("Template 'device_registration_email.html' not found in cache! Using minimal HTML fallback.");
        format!(
            "<html><body><h2>Device Registration Successful!</h2><p>Dear {}, your device {} (Serial: {}) has been successfully registered.</p><p>Country: {}</p><p>Province: {}</p><p>Address: {}</p><p>Warranty Status: {}</p><p>Registration Date: {}</p></body></html>",
            escaped_name, escaped_product, escaped_serial, escaped_country, escaped_province, escaped_address, escaped_warranty, escaped_date
        )
    };

    let email_payload = serde_json::json!({
        "to": recipient_email,
        "subject": "Zent Device Registration Confirmation",
        "body": email_body
    });

    publish_email_task(rabbitmq_connection, email_payload, "device registration email").await
}

/// Generic send_email helper — publishes arbitrary email content via RabbitMQ.
/// Used by cleanup/cancel flows that don't need templated emails.
pub async fn send_email(
    rabbitmq_connection: &Arc<Connection>,
    recipient_email: &str,
    subject: &str,
    body: &str,
) -> Result<(), AppError> {
    let email_payload = serde_json::json!({
        "to": recipient_email,
        "subject": subject,
        "body": body,
    });

    publish_email_task(rabbitmq_connection, email_payload, "generic email").await
}
