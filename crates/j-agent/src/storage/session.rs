pub mod meta;
pub mod paths;
pub mod transcript;

// Re-export all public API so external callers are unaffected.
pub use meta::{
    SessionMeta, SessionMetaFile, delete_session, generate_session_id, list_sessions,
    load_session_meta_file, save_session_meta_file, write_session_metrics,
};
pub use paths::{SessionPaths, session_file_path, sessions_dir};
pub use transcript::{
    append_event_to_path, append_session_event, append_session_op, find_latest_session_id,
    load_display_session, load_session, load_session_ops, read_transcript_with_timestamps,
};
