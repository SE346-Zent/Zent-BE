use prelude::*;

pub async fn get_profile(
    State(db): State<DatabaseConnection>,
    State(encoding_key): State<EncodingKey>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<UserProfileResponse>, AppError> {
    
}
