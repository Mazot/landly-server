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
    }
}

diesel::joinable!(chats -> countries_connections (origin_country_connection_id));
diesel::joinable!(countries_connections -> countries (location_country_id));
diesel::joinable!(countries_to_languages -> countries (country_id));
diesel::joinable!(countries_to_languages -> languages (language_id));
diesel::joinable!(organisations -> countries (location_country_id));
diesel::joinable!(organisations -> organisation_types (organisation_type_id));

diesel::allow_tables_to_appear_in_same_query!(
    chats,
    countries,
    countries_connections,
    countries_to_languages,
    languages,
    organisation_types,
    organisations,
);
