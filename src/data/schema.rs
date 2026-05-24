// @generated automatically by Diesel CLI.

diesel::table! {
    chats (id) {
        id -> Uuid,
        app -> Nullable<Text>,
        origin_country_connection_id -> Nullable<Uuid>,
        link -> Nullable<Text>,
        info -> Nullable<Text>,
    }
}

diesel::table! {
    countries (id) {
        id -> Uuid,
        name -> Text,
        geo_json -> Nullable<Jsonb>,
        flag -> Nullable<Text>,
        capital_city -> Nullable<Text>,
        description -> Nullable<Text>,
    }
}

diesel::table! {
    countries_connections (id) {
        id -> Uuid,
        embassy_org_id -> Nullable<Uuid>,
        consulate_org_id -> Nullable<Uuid>,
        common_info -> Nullable<Text>,
        location_country_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    countries_to_languages (country_id, language_id) {
        country_id -> Uuid,
        language_id -> Uuid,
    }
}

diesel::table! {
    images (id) {
        id -> Uuid,
        organisation_id -> Uuid,
        uploaded_by -> Uuid,
        #[max_length = 512]
        s3_key -> Varchar,
        #[max_length = 255]
        s3_bucket -> Varchar,
        #[max_length = 255]
        file_name -> Varchar,
        #[max_length = 100]
        content_type -> Varchar,
        file_size -> Int8,
        width -> Nullable<Int4>,
        height -> Nullable<Int4>,
        is_primary -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    languages (id) {
        id -> Uuid,
        name -> Text,
        symbol -> Nullable<Text>,
    }
}

diesel::table! {
    organisation_types (id) {
        id -> Uuid,
        #[sql_name = "type"]
        type_ -> Text,
        color -> Nullable<Text>,
        #[max_length = 255]
        title -> Nullable<Varchar>,
    }
}

diesel::table! {
    organisations (id) {
        id -> Uuid,
        name -> Text,
        tel -> Nullable<Text>,
        email -> Nullable<Text>,
        address -> Nullable<Text>,
        description -> Nullable<Text>,
        location_country_id -> Nullable<Uuid>,
        organisation_type_id -> Nullable<Uuid>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        latitude -> Nullable<Numeric>,
        longitude -> Nullable<Numeric>,
        founder_country_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    user_providers (id) {
        id -> Uuid,
        user_id -> Uuid,
        #[max_length = 128]
        provider -> Varchar,
        #[max_length = 128]
        provider_user_id -> Varchar,
        #[max_length = 255]
        email -> Nullable<Varchar>,
        created_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        #[max_length = 50]
        username -> Varchar,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 255]
        password_hash -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    users_to_languages (user_id, language_id) {
        user_id -> Uuid,
        language_id -> Uuid,
    }
}

diesel::joinable!(chats -> countries_connections (origin_country_connection_id));
diesel::joinable!(countries_connections -> countries (location_country_id));
diesel::joinable!(countries_to_languages -> countries (country_id));
diesel::joinable!(countries_to_languages -> languages (language_id));
diesel::joinable!(images -> organisations (organisation_id));
diesel::joinable!(images -> users (uploaded_by));
diesel::joinable!(organisations -> organisation_types (organisation_type_id));
diesel::joinable!(user_providers -> users (user_id));
diesel::joinable!(users_to_languages -> languages (language_id));
diesel::joinable!(users_to_languages -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    chats,
    countries,
    countries_connections,
    countries_to_languages,
    images,
    languages,
    organisation_types,
    organisations,
    user_providers,
    users,
    users_to_languages,
);
