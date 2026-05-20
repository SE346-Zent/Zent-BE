use zent_be::model::jwt_claims::Claims;
use serde_json;

#[test]
fn test_jwt_claims_serialization() {
    let claims = Claims {
        sub: "user123".to_string(),
        iat: 1234567890,
        exp: 1234567890 + 3600,
    };
    
    let json = serde_json::to_string(&claims).unwrap();
    let deserialized: Claims = serde_json::from_str(&json).unwrap();
    
    assert_eq!(claims.sub, deserialized.sub);
    assert_eq!(claims.iat, deserialized.iat);
    assert_eq!(claims.exp, deserialized.exp);
}
