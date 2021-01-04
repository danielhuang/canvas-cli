use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasAssignment {
    pub id: i64,
    pub description: Option<String>,
    pub due_at: Option<DateTime<Local>>,
    pub unlock_at: Option<DateTime<Local>>,
    pub lock_at: Option<DateTime<Local>>,
    pub points_possible: Option<f64>,
    pub grading_type: String,
    pub assignment_group_id: i64,
    pub grading_standard_id: Value,
    pub created_at: String,
    pub updated_at: String,
    pub peer_reviews: bool,
    pub automatic_peer_reviews: bool,
    pub position: i64,
    pub grade_group_students_individually: bool,
    pub anonymous_peer_reviews: bool,
    pub group_category_id: Value,
    pub post_to_sis: bool,
    pub moderated_grading: bool,
    pub omit_from_final_grade: bool,
    pub intra_group_peer_reviews: bool,
    pub anonymous_instructor_annotations: bool,
    pub anonymous_grading: bool,
    pub graders_anonymous_to_graders: bool,
    pub grader_count: i64,
    pub grader_comments_visible_to_graders: bool,
    pub final_grader_id: Value,
    pub grader_names_visible_to_final_grader: bool,
    pub allowed_attempts: i64,
    pub secure_params: String,
    pub course_id: i64,
    pub name: String,
    pub submission_types: Vec<String>,
    pub has_submitted_submissions: bool,
    pub due_date_required: bool,
    pub max_name_length: i64,
    pub in_closed_grading_period: bool,
    pub is_quiz_assignment: bool,
    pub can_duplicate: bool,
    pub original_course_id: Option<i64>,
    pub original_assignment_id: Option<i64>,
    pub original_assignment_name: Option<String>,
    pub original_quiz_id: Value,
    pub workflow_state: String,
    pub muted: bool,
    pub html_url: String,
    pub published: bool,
    pub only_visible_to_overrides: bool,
    pub submission: Option<Submission>,
    pub locked_for_user: bool,
    pub submissions_download_url: String,
    pub post_manually: bool,
    pub anonymize_students: bool,
    pub require_lockdown_browser: bool,
    pub external_tool_tag_attributes: Option<ExternalToolTagAttributes>,
    pub url: Option<String>,
    pub is_quiz_lti_assignment: Option<bool>,
    #[serde(default)]
    pub frozen_attributes: Vec<String>,
    pub discussion_topic: Option<DiscussionTopic>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Submission {
    pub id: i64,
    pub body: Option<String>,
    pub url: Option<String>,
    pub grade: Option<String>,
    pub score: Option<f64>,
    pub submitted_at: Option<String>,
    pub assignment_id: i64,
    pub user_id: i64,
    pub submission_type: Option<String>,
    pub workflow_state: String,
    pub grade_matches_current_submission: bool,
    pub graded_at: Option<String>,
    pub grader_id: Option<i64>,
    pub attempt: Option<i64>,
    pub cached_due_date: Option<String>,
    pub excused: Option<bool>,
    pub late_policy_status: Value,
    pub points_deducted: Option<f64>,
    pub grading_period_id: Option<i64>,
    pub extra_attempts: Value,
    pub posted_at: Option<String>,
    pub late: bool,
    pub missing: bool,
    pub seconds_late: i64,
    pub entered_grade: Option<String>,
    pub entered_score: Option<f64>,
    pub preview_url: String,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    pub external_tool_url: Option<String>,
    pub media_comment: Option<MediaComment>,
    #[serde(default)]
    pub discussion_entries: Vec<DiscussionEntry>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attachment {
    pub id: i64,
    pub uuid: String,
    pub folder_id: Option<i64>,
    pub display_name: String,
    pub filename: String,
    pub upload_status: String,
    #[serde(rename = "content-type")]
    pub content_type: String,
    pub url: String,
    pub size: i64,
    pub created_at: String,
    pub updated_at: String,
    pub unlock_at: Value,
    pub locked: bool,
    pub hidden: bool,
    pub lock_at: Value,
    pub hidden_for_user: bool,
    pub thumbnail_url: Option<String>,
    pub modified_at: String,
    pub mime_class: String,
    pub media_entry_id: Value,
    pub locked_for_user: bool,
    pub preview_url: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaComment {
    #[serde(rename = "content-type")]
    pub content_type: String,
    pub display_name: Value,
    pub media_id: String,
    pub media_type: String,
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscussionEntry {
    pub id: i64,
    pub user_id: i64,
    pub parent_id: Value,
    pub created_at: String,
    pub updated_at: String,
    pub rating_count: Value,
    pub rating_sum: Value,
    pub user_name: String,
    pub message: String,
    pub user: User,
    pub read_state: String,
    pub forced_read_state: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub display_name: String,
    pub avatar_image_url: String,
    pub html_url: String,
    pub pronouns: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalToolTagAttributes {
    pub url: String,
    pub new_tab: Option<bool>,
    pub resource_link_id: String,
    pub external_data: String,
    pub content_type: String,
    pub content_id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscussionTopic {
    pub id: i64,
    pub title: String,
    pub last_reply_at: String,
    pub created_at: String,
    pub delayed_post_at: Value,
    pub posted_at: String,
    pub assignment_id: i64,
    pub root_topic_id: Value,
    pub position: Value,
    pub podcast_has_student_posts: bool,
    pub discussion_type: String,
    pub lock_at: Value,
    pub allow_rating: bool,
    pub only_graders_can_rate: bool,
    pub sort_by_rating: bool,
    pub is_section_specific: bool,
    pub user_name: Value,
    pub discussion_subentry_count: i64,
    pub permissions: Permissions,
    pub require_initial_post: bool,
    pub user_can_see_posts: bool,
    pub podcast_url: Value,
    pub read_state: String,
    pub unread_count: i64,
    pub subscribed: bool,
    pub attachments: Vec<Value>,
    pub published: bool,
    pub can_unpublish: bool,
    pub locked: bool,
    pub can_lock: bool,
    pub comments_disabled: bool,
    pub author: Author,
    pub html_url: String,
    pub url: String,
    pub pinned: bool,
    pub group_category_id: Value,
    pub can_group: bool,
    pub topic_children: Vec<Value>,
    pub group_topic_children: Vec<Value>,
    pub locked_for_user: bool,
    pub message: String,
    pub todo_date: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Permissions {
    pub attach: bool,
    pub update: bool,
    pub reply: bool,
    pub delete: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Author {}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasCourse {
    pub id: i64,
    pub name: String,
    pub account_id: i64,
    pub uuid: String,
    pub start_at: String,
    pub grading_standard_id: Value,
    pub is_public: Option<bool>,
    pub created_at: String,
    pub course_code: String,
    pub default_view: String,
    pub root_account_id: i64,
    pub enrollment_term_id: i64,
    pub license: Option<String>,
    pub grade_passback_setting: Value,
    pub end_at: Value,
    pub public_syllabus: bool,
    pub public_syllabus_to_auth: bool,
    pub storage_quota_mb: i64,
    pub is_public_to_auth_users: bool,
    pub apply_assignment_group_weights: bool,
    pub calendar: Calendar,
    pub time_zone: String,
    pub blueprint: bool,
    pub enrollments: Vec<Enrollment>,
    pub hide_final_grades: bool,
    pub workflow_state: String,
    pub restrict_enrollments_to_course_dates: bool,
    pub overridden_course_visibility: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Calendar {
    pub ics: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Enrollment {
    #[serde(rename = "type")]
    pub type_field: String,
    pub role: String,
    pub role_id: i64,
    pub user_id: i64,
    pub enrollment_state: String,
    pub limit_privileges_to_course_section: bool,
}
