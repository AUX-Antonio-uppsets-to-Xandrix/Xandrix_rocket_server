// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Nullable<Integer>,
        user_id -> Text,
        password -> Text,
        user_image_url -> Text,
        grayscale -> Integer,
        brightness -> Integer,
        threshold -> Integer,
        rotation -> Integer,
    }
}
