use app::{JobGui, JobGuiState};

#[test]
fn app_reexports_job_gui_components_for_embedding() {
    let _gui = JobGui::new();
    let state = JobGuiState::new();

    assert_eq!(state.selected_job_id(), None);
}
