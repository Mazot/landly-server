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
    corridors (id) {
        id -> Uuid,
        user_id -> Uuid,
        from_country_id -> Uuid,
        to_country_id -> Uuid,
        is_default -> Bool,
        created_at -> Timestamp,
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
        currency -> Nullable<Text>,
        phone_code -> Nullable<Text>,
        top_cities -> Nullable<Jsonb>,
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
    moderation_events (id) {
        id -> Uuid,
        target_kind -> Text,
        target_id -> Uuid,
        moderator_id -> Nullable<Uuid>,
        action -> Text,
        note -> Nullable<Text>,
        flags -> Nullable<Jsonb>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    org_checkins (id) {
        id -> Uuid,
        organisation_id -> Uuid,
        user_id -> Uuid,
        still_active -> Bool,
        tip -> Nullable<Text>,
        created_at -> Timestamp,
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
        slug -> Nullable<Text>,
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
        created_by -> Nullable<Uuid>,
        verified -> Bool,
        status -> Text,
        moderation_note -> Nullable<Text>,
        added_by -> Nullable<Text>,
        city -> Nullable<Text>,
        website -> Nullable<Text>,
        telegram -> Nullable<Text>,
        whatsapp -> Nullable<Text>,
        services -> Array<Nullable<Text>>,
        languages -> Array<Nullable<Text>>,
        opening_hours -> Nullable<Jsonb>,
        timezone -> Nullable<Text>,
        cost -> Nullable<Text>,
        google_place_id -> Nullable<Text>,
        google_rating -> Nullable<Float8>,
        visits_count -> Int8,
        rating_avg -> Nullable<Float8>,
        reviews_count -> Int8,
    }
}

diesel::table! {
    people (id) {
        id -> Uuid,
        name -> Text,
        bio -> Nullable<Text>,
        city -> Nullable<Text>,
        location_country_id -> Nullable<Uuid>,
        skills -> Array<Nullable<Text>>,
        email -> Nullable<Text>,
        whatsapp -> Nullable<Text>,
        send_via -> Nullable<Text>,
        consent_given -> Bool,
        status -> Text,
        show_whatsapp -> Bool,
        show_email -> Bool,
        show_city -> Bool,
        allow_reviews -> Bool,
        recommended_by -> Nullable<Uuid>,
        claimed_by -> Nullable<Uuid>,
        moderation_note -> Nullable<Text>,
        rating_avg -> Nullable<Float8>,
        reviews_count -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    people_to_languages (person_id, language_id) {
        person_id -> Uuid,
        language_id -> Uuid,
    }
}

diesel::table! {
    person_claim_tokens (id) {
        id -> Uuid,
        person_id -> Uuid,
        token -> Text,
        expires_at -> Timestamp,
        used_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    person_vouches (id) {
        id -> Uuid,
        person_id -> Uuid,
        user_id -> Uuid,
        note -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    reports (id) {
        id -> Uuid,
        reporter_id -> Nullable<Uuid>,
        target_kind -> Text,
        target_id -> Uuid,
        reason -> Text,
        status -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    reviews (id) {
        id -> Uuid,
        author_id -> Uuid,
        organisation_id -> Nullable<Uuid>,
        person_id -> Nullable<Uuid>,
        rating -> Int4,
        topic -> Nullable<Text>,
        text -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    saved_items (id) {
        id -> Uuid,
        user_id -> Uuid,
        kind -> Text,
        target_id -> Uuid,
        note -> Nullable<Text>,
        list_name -> Nullable<Text>,
        created_at -> Timestamp,
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
        name -> Nullable<Text>,
        bio -> Nullable<Text>,
        city -> Nullable<Text>,
        home_country_id -> Nullable<Uuid>,
        avatar_color -> Nullable<Text>,
        locale -> Text,
        here_as -> Nullable<Text>,
        role -> Text,
        notification_settings -> Nullable<Jsonb>,
    }
}

diesel::table! {
    users_to_languages (user_id, language_id) {
        user_id -> Uuid,
        language_id -> Uuid,
    }
}

diesel::joinable!(chats -> countries_connections (origin_country_connection_id));
diesel::joinable!(corridors -> users (user_id));
diesel::joinable!(countries_connections -> countries (location_country_id));
diesel::joinable!(countries_to_languages -> countries (country_id));
diesel::joinable!(countries_to_languages -> languages (language_id));
diesel::joinable!(images -> organisations (organisation_id));
diesel::joinable!(images -> users (uploaded_by));
diesel::joinable!(moderation_events -> users (moderator_id));
diesel::joinable!(org_checkins -> organisations (organisation_id));
diesel::joinable!(org_checkins -> users (user_id));
diesel::joinable!(organisations -> organisation_types (organisation_type_id));
diesel::joinable!(organisations -> users (created_by));
diesel::joinable!(people -> countries (location_country_id));
diesel::joinable!(people_to_languages -> languages (language_id));
diesel::joinable!(people_to_languages -> people (person_id));
diesel::joinable!(person_claim_tokens -> people (person_id));
diesel::joinable!(person_vouches -> people (person_id));
diesel::joinable!(person_vouches -> users (user_id));
diesel::joinable!(reports -> users (reporter_id));
diesel::joinable!(reviews -> organisations (organisation_id));
diesel::joinable!(reviews -> people (person_id));
diesel::joinable!(reviews -> users (author_id));
diesel::joinable!(saved_items -> users (user_id));
diesel::joinable!(user_providers -> users (user_id));
diesel::joinable!(users -> countries (home_country_id));
diesel::joinable!(users_to_languages -> languages (language_id));
diesel::joinable!(users_to_languages -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    chats,
    corridors,
    countries,
    countries_connections,
    countries_to_languages,
    images,
    languages,
    moderation_events,
    org_checkins,
    organisation_types,
    organisations,
    people,
    people_to_languages,
    person_claim_tokens,
    person_vouches,
    reports,
    reviews,
    saved_items,
    user_providers,
    users,
    users_to_languages,
);
