// Local modules that depend on kild-core internals
pub mod handler;
pub mod overlaps;

// Re-export kild-git submodules for consumer compatibility
pub use kild_git::{
    cli, errors, health, naming, project, query, remote, removal, status, test_support, types,
    validation,
};

// Re-export commonly used types and functions from kild-git
pub use kild_git::{
    // types
    BaseBranchDrift,
    BranchHealth,
    CleanKild,
    CommitActivity,
    ConflictStatus,
    DiffStats,
    FileOverlap,
    // errors
    GitError,
    GitStats,
    // naming
    KILD_BRANCH_PREFIX,
    OverlapReport,
    UncommittedDetails,
    // query
    WorktreeEntry,
    WorktreeStatus,
    calculate_worktree_path,
    // health
    collect_branch_health,
    // status
    collect_git_stats,
    // removal
    delete_branch_if_exists,
    delete_local_branch,
    derive_project_name_from_path,
    derive_project_name_from_remote,
    // project
    detect_project,
    detect_project_at,
    ensure_in_repo,
    // remote
    fetch_remote,
    find_main_repo_root,
    generate_project_id,
    // validation
    get_current_branch,
    get_diff_stats,
    get_origin_url,
    get_worktree_status,
    has_any_remote,
    has_uncommitted_changes,
    head_branch_name,
    is_git_repo,
    is_valid_git_directory,
    is_worktree_valid,
    kild_branch_name,
    kild_worktree_admin_name,
    list_local_branch_names,
    list_worktree_entries,
    rebase_worktree,
    remove_worktree,
    remove_worktree_by_path,
    remove_worktree_force,
    sanitize_for_path,
    should_use_current_branch,
    validate_branch_name,
    validate_git_arg,
    worktree_active_branches,
};

// Local re-exports
pub use handler::create_worktree;
pub use overlaps::collect_file_overlaps;
