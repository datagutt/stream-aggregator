// @generated automatically by Diesel CLI.

diesel::table! {
    discovery_rules (id) {
        id -> Text,
        name -> Text,
        platform -> Text,
        enabled -> Bool,
        filters -> Text,
        interval_secs -> Integer,
        apply_labels -> Text,
        apply_group -> Nullable<Text>,
        created_at -> Text,
        last_run_at -> Nullable<Text>,
    }
}

diesel::table! {
    streams (id) {
        id -> Text,
        platform -> Text,
        user_id -> Text,
        display_name -> Text,
        avatar_url -> Nullable<Text>,
        is_live -> Bool,
        title -> Nullable<Text>,
        viewer_count -> Nullable<Integer>,
        thumbnail_url -> Nullable<Text>,
        category -> Nullable<Text>,
        tags -> Text,
        language -> Nullable<Text>,
        started_at -> Nullable<Text>,
        last_fetched_at -> Text,
        last_live_at -> Nullable<Text>,
        metadata -> Text,
    }
}

diesel::table! {
    tracked_streamers (platform, user_id) {
        platform -> Text,
        user_id -> Text,
        custom_name -> Nullable<Text>,
        group_name -> Nullable<Text>,
        priority -> Nullable<Integer>,
        labels -> Text,
        source -> Text,
        discovery_rule_id -> Nullable<Text>,
        created_at -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(discovery_rules, streams, tracked_streamers,);
