/// Generates a standardized API response struct with Utoipa OpenAPI schemas.
#[macro_export]
macro_rules! define_api_response {
    (
        $struct_name:ident,
        $(#[$data_attr:meta])* $data_type:ty,
        $(#[$meta_attr:meta])* $meta_type:ty
    ) => {
        #[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
        #[serde(rename_all = "camelCase")]
        pub struct $struct_name {
            #[schema(example = 200)]
            /// HTTP status code indicator.
            pub status_code: u16,
            #[schema(example = "Success")]
            /// Message describing the response.
            pub message: String,

            $(#[$data_attr])*
            /// Response data.
            pub data: $data_type,

            $(#[$meta_attr])*
            /// Response metadata.
            pub meta: $meta_type,
        }
    };
}
