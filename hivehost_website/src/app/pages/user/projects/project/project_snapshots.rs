use crate::app::components::csrf_field::CSRFField;
use crate::app::pages::user::projects::project::project_snapshots::server_fns::{
    CreateProjectSnapshot, DeleteProjectSnapshot, SetActiveProjectSnapshot,
    UnsetActiveProjectSnapshot,
};
use crate::app::pages::user::projects::project::ProjectSlugSignal;
use crate::app::pages::{GlobalState, GlobalStateStoreFields};
use crate::app::IntoView;
use leptos::either::{Either, EitherOf3};
use leptos::html::{Input, Textarea};
use leptos::prelude::*;
use reactive_stores::Store;
use time::format_description::well_known::Rfc3339;
use web_sys::SubmitEvent;

#[component]
pub fn ProjectSnapshots() -> impl IntoView {
    let global_state: Store<GlobalState> = expect_context();
    let project_slug_signal: Signal<ProjectSlugSignal> = expect_context();

    let permission_signal = Signal::derive(move || {
        global_state
            .project()
            .get()
            .map(|p| p.1)
            .unwrap_or_default()
    });

    let slug_signal = Signal::derive(move || project_slug_signal.get().0);
    let csrf_signal = Signal::derive(move || global_state.csrf().get());
    let active_snapshot_id_signal = Signal::derive(move || {
        global_state
            .project()
            .get()
            .and_then(|p| p.2.active_snapshot_id)
    });

    // --- Actions ---
    let create_snapshot_action = ServerAction::<CreateProjectSnapshot>::new();
    let delete_snapshot_action = ServerAction::<DeleteProjectSnapshot>::new();
    let set_active_snapshot_action = ServerAction::<SetActiveProjectSnapshot>::new();
    let unset_active_snapshot_action = ServerAction::<UnsetActiveProjectSnapshot>::new();

    
    // --- Resource for Snapshots List ---
    let snapshots_resource = Resource::new(
        move || {
            (
                slug_signal.get(),
                create_snapshot_action.version().get(),
                delete_snapshot_action.version().get(),
                set_active_snapshot_action.version().get(),
                unset_active_snapshot_action.version().get(),
            )
        },
        move |(slug, _, _, _, _)| async move { server_fns::get_project_snapshots(slug).await },
    );

    // --- Form State ---
    let snapshot_name_ref = NodeRef::<Input>::new();
    let snapshot_description_ref = NodeRef::<Textarea>::new();

    // --- Feedback Signals ---
    let (create_feedback, set_create_feedback) = signal(String::new());
    let (delete_feedback, set_delete_feedback) = signal(String::new());
    let (set_active_feedback, set_set_active_feedback) = signal(String::new());
    let (unset_active_feedback, set_unset_active_feedback) = signal(String::new());

    // --- Effects for Feedback ---
    Effect::new(move |_| {
        if let Some(result) = create_snapshot_action.value().get() {
            match result {
                Ok(_) => {
                    set_create_feedback.set("Snapshot created successfully.".to_string());
                    // Clear form fields after successful creation
                    if let Some(input) = snapshot_name_ref.get() {
                        input.set_value("");
                    }
                    if let Some(textarea) = snapshot_description_ref.get() {
                        textarea.set_value("");
                    }
                }
                Err(e) => set_create_feedback.set(format!("Error creating snapshot: {e}")),
            }
        }
    });

    Effect::new(move |_| {
        if let Some(result) = delete_snapshot_action.value().get() {
            match result {
                Ok(_) => set_delete_feedback.set("Snapshot deleted successfully.".to_string()),
                Err(e) => set_delete_feedback.set(format!("Error deleting snapshot: {e}")),
            }
        } else {
            set_delete_feedback.set("".to_string()); // Clear on potential refetch
        }
    });

    Effect::new(move |_| {
        if let Some(result) = set_active_snapshot_action.value().get() {
            match result {
                Ok(_) => set_set_active_feedback.set("Snapshot set as active.".to_string()),
                Err(e) => {
                    set_set_active_feedback.set(format!("Error setting active snapshot: {e}"))
                }
            }
        } else {
            set_set_active_feedback.set("".to_string()); // Clear on potential refetch
        }
    });

    Effect::new(move |_| {
        if let Some(result) = unset_active_snapshot_action.value().get() {
            match result {
                Ok(_) => set_unset_active_feedback.set("Active snapshot unset.".to_string()),
                Err(e) => {
                    set_unset_active_feedback.set(format!("Error unsetting active snapshot: {e}"))
                }
            }
        } else {
            set_unset_active_feedback.set("".to_string()); // Clear on potential refetch
        }
    });
    
    let on_set_active_submit = move |ev: SubmitEvent| {
        let form = SetActiveProjectSnapshot::from_event(&ev);
        ev.prevent_default();
        if form.is_err(){
            return;
        }
        let data = form.unwrap();
        let snapshot_id = data.snapshot_id;
        set_active_snapshot_action.dispatch(data);
        global_state.project().update(|project_opt| {
            match project_opt{
                None => {}
                Some((_,_,project)) => {
                    project.active_snapshot_id = Some(snapshot_id);
                }
            }
        });
    };
    let on_unset_active_submit = move |ev: SubmitEvent| {
        let form = UnsetActiveProjectSnapshot::from_event(&ev);
        ev.prevent_default();
        if form.is_err(){
            return;
        }
        unset_active_snapshot_action.dispatch(form.unwrap());
        global_state.project().update(|project_opt| {
            match project_opt{
                None => {}
                Some((_,_,project)) => {
                    project.active_snapshot_id = None;
                }
            }
        });
    };
    

    // --- Event Handlers ---
    let on_create_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        let name = snapshot_name_ref.get().expect("name input exists").value();
        let description = snapshot_description_ref.get().map(|el| el.value());
        let csrf = csrf_signal.get().unwrap_or_default();
        let slug = slug_signal.get();

        if name.trim().is_empty() {
            set_create_feedback.set("Snapshot name cannot be empty.".to_string());
            return;
        }
        set_create_feedback.set("Creating...".to_string()); // Indicate processing
        create_snapshot_action.dispatch(CreateProjectSnapshot {
            csrf,
            project_slug: slug,
            name,
            description: description.filter(|d| !d.trim().is_empty()),
        });
    };

    let on_delete_submit = move |ev: SubmitEvent, snapshot_name: String| {
        let confirmed = if let Some(window) = web_sys::window() {
            window
                .confirm_with_message(&format!(
                    "Are you sure you want to delete snapshot '{snapshot_name}'?"
                ))
                .unwrap_or(false)
        } else {
            false
        };
        if !confirmed {
            ev.prevent_default();
        }
    };

    view! {
        <div class="space-y-10">
            // --- Create Snapshot Section ---
            <div class="section-border" class=("hidden", move || !permission_signal().is_owner())>
                <h2 class="section-title">"Create New Snapshot"</h2>
                <p class="section-desc">"Create a snapshot of the current project state."</p>
                <form on:submit=on_create_submit class="mt-6 space-y-4">
                    <CSRFField />
                    <div>
                        <label for="snapshot_name" class="form-label">
                            "Snapshot Name"
                        </label>
                        <div class="mt-2">
                            <input
                                type="text"
                                name="snapshot_name"
                                id="snapshot_name"
                                node_ref=snapshot_name_ref
                                class="form-input"
                                required
                                placeholder="e.g., v1.0-release"
                            />
                        </div>
                    </div>
                    <div>
                        <label for="snapshot_description" class="form-label">
                            "Description (Optional)"
                        </label>
                        <div class="mt-2">
                            <textarea
                                name="snapshot_description"
                                id="snapshot_description"
                                node_ref=snapshot_description_ref
                                rows="3"
                                class="block w-full rounded-md bg-white/5 px-3 py-1.5 text-base text-white outline-1 -outline-offset-1 outline-white/10 placeholder:text-gray-500 focus:outline-2 focus:-outline-offset-2 focus:outline-indigo-500 sm:text-sm/6"
                                placeholder="Describe the changes in this snapshot..."
                            ></textarea>
                        </div>
                    </div>
                    <div class="flex items-center justify-end gap-x-6">
                        <div class="text-sm mr-auto min-h-[1.25em]">{create_feedback}</div>
                        <button
                            type="submit"
                            class="btn btn-primary"
                            disabled=move || create_snapshot_action.pending().get()
                        >
                            "Create Snapshot"
                        </button>
                    </div>
                </form>
            </div>

            // --- List Snapshots Section ---
            // No border-b here, it's the last section
            <div>
                <h2 class="section-title">"Existing Snapshots"</h2>
                <p class="section-desc">
                    "Manage your project snapshots. At most 20 snapshots are kept."
                </p>

                <div class="mt-2 text-sm text-right min-h-[1.25em] text-yellow-400">
                    {delete_feedback}
                </div>
                <div class="mt-1 text-sm text-right min-h-[1.25em] text-green-400">
                    {set_active_feedback}
                </div>
                <div class="mt-1 text-sm text-right min-h-[1.25em] text-yellow-400">
                    {unset_active_feedback}
                </div>

                <div class="mt-6 flow-root">
                    <div class="-mx-4 -my-2 overflow-x-auto sm:-mx-6 lg:-mx-8">
                        <div class="inline-block min-w-full py-2 align-middle sm:px-6 lg:px-8">
                            <Transition fallback=move || {
                                view! { <p class="text-gray-400">"Loading snapshots..."</p> }
                            }>
                                {move || {
                                    Suspend::new(async move {
                                        match snapshots_resource.get() {
                                            Some(Ok(snapshots)) => {
                                                if snapshots.is_empty() {
                                                    EitherOf3::A(
                                                        view! {
                                                            <p class="text-gray-400 mt-4">
                                                                "No snapshots created yet."
                                                            </p>
                                                        },
                                                    )
                                                } else {
                                                    EitherOf3::B(
                                                        view! {
                                                            <table class="table">
                                                                <thead>
                                                                    <tr>
                                                                        <th scope="col" class="table-th">
                                                                            "Name"
                                                                        </th>
                                                                        <th scope="col" class="table-th">
                                                                            "Description"
                                                                        </th>
                                                                        <th scope="col" class="table-th">
                                                                            "Created At"
                                                                        </th>
                                                                        <th scope="col" class="relative py-3.5 pl-3 pr-4 sm:pr-0">
                                                                            <span class="sr-only">Actions</span>
                                                                        </th>
                                                                    </tr>
                                                                </thead>
                                                                <tbody class="divide-y divide-gray-800">
                                                                    <For
                                                                        each=move || snapshots.clone()
                                                                        key=|snapshot| snapshot.id
                                                                        children=move |snapshot| {
                                                                            let is_active = active_snapshot_id_signal.get()
                                                                                == Some(snapshot.id);
                                                                            let (is_active_signal, _) = signal(is_active);
                                                                            let created_at_formatted = snapshot
                                                                                .created_at
                                                                                .format(&Rfc3339)
                                                                                .unwrap_or_default();
                                                                            let (name_signal, _) = signal(
                                                                                snapshot.snapshot_name.clone(),
                                                                            );
                                                                            let (id_signal, _) = signal(snapshot.id);

                                                                            view! {
                                                                                <tr>
                                                                                    <td class="table-td font-medium whitespace-nowrap">
                                                                                        {snapshot.name.clone()}
                                                                                        {move || {
                                                                                            is_active
                                                                                                .then(|| {
                                                                                                    view! {
                                                                                                        <span class="ml-2 inline-flex items-center rounded-full bg-green-900 px-2 py-0.5 text-xs font-medium text-green-300 ring-1 ring-inset ring-green-500/10">
                                                                                                            Active
                                                                                                        </span>
                                                                                                    }
                                                                                                })
                                                                                        }}
                                                                                    </td>
                                                                                    <td class="table-td text-gray-400">
                                                                                        {snapshot.description.clone().unwrap_or_default()}
                                                                                    </td>
                                                                                    <td class="table-td text-gray-400 whitespace-nowrap">
                                                                                        {created_at_formatted}
                                                                                    </td>
                                                                                    <td class="relative whitespace-nowrap py-4 pl-3 pr-4 text-right text-sm font-medium sm:pr-0">
                                                                                        <div
                                                                                            class="flex justify-end items-center space-x-2"
                                                                                            class=("hidden", move || !permission_signal().is_owner())
                                                                                        >
                                                                                            // Set Active Button
                                                                                            {move || match is_active_signal() {
                                                                                                true => {
                                                                                                    Either::Left(

                                                                                                        view! {
                                                                                                            <ActionForm action=unset_active_snapshot_action on:submit=on_unset_active_submit>
                                                                                                                <CSRFField />
                                                                                                                <input
                                                                                                                    type="hidden"
                                                                                                                    name="project_slug"
                                                                                                                    value=slug_signal.get()
                                                                                                                />
                                                                                                                <button
                                                                                                                    class="btn btn-danger"
                                                                                                                    disabled=move || {
                                                                                                                        unset_active_snapshot_action.pending().get()
                                                                                                                    }
                                                                                                                >
                                                                                                                    "Unset Active"
                                                                                                                </button>
                                                                                                            </ActionForm>
                                                                                                        },
                                                                                                    )
                                                                                                }
                                                                                                false => {
                                                                                                    Either::Right(
                                                                                                        view! {
                                                                                                            <ActionForm action=set_active_snapshot_action on:submit=on_set_active_submit>
                                                                                                                <CSRFField />
                                                                                                                <input
                                                                                                                    type="hidden"
                                                                                                                    name="project_slug"
                                                                                                                    value=slug_signal.get()
                                                                                                                />
                                                                                                                <input type="hidden" name="snapshot_id" value=snapshot.id />
                                                                                                                <button
                                                                                                                    class="btn btn-success"
                                                                                                                    disabled=move || set_active_snapshot_action.pending().get()
                                                                                                                >
                                                                                                                    "Set Active"
                                                                                                                </button>
                                                                                                            </ActionForm>
                                                                                                        },
                                                                                                    )
                                                                                                }
                                                                                            }}
                                                                                            // Delete Button (using form for potential future hidden fields)
                                                                                            <ActionForm
                                                                                                action=delete_snapshot_action
                                                                                                on:submit=move |ev| on_delete_submit(ev, name_signal())
                                                                                            >
                                                                                                <CSRFField />
                                                                                                <input
                                                                                                    type="hidden"
                                                                                                    name="project_slug"
                                                                                                    value=slug_signal.get()
                                                                                                />
                                                                                                <input type="hidden" name="snapshot_id" value=snapshot.id />
                                                                                                <button
                                                                                                    type="submit"
                                                                                                    class="btn btn-danger"
                                                                                                    disabled=move || {
                                                                                                        delete_snapshot_action.pending().get()
                                                                                                            || active_snapshot_id_signal.get() == Some(id_signal())
                                                                                                    }
                                                                                                >
                                                                                                    "Delete"
                                                                                                </button>
                                                                                            </ActionForm>
                                                                                        </div>
                                                                                    </td>
                                                                                </tr>
                                                                            }
                                                                        }
                                                                    />
                                                                </tbody>
                                                            </table>
                                                        },
                                                    )
                                                }
                                            }
                                            _ => EitherOf3::C(()),
                                        }
                                    })
                                }}
                            </Transition>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

// --- Server Functions ---
pub mod server_fns {
    use leptos::server;

    use crate::models::ProjectSnapshot;
    use crate::AppResult;
    use common::ProjectSlugStr;

    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
        use crate::security::permission::ssr::handle_project_permission_request;
        use crate::api::ssr::{request_server_project_action, request_hosting_action};
        use common::hosting::HostingAction;
        use common::website_to_server::permission::Permission;
        use common::website_to_server::server_project_action::snapshot::ServerProjectSnapshotAction;
        use crate::AppError;
    }}

    #[server]
    pub async fn get_project_snapshots(
        project_slug: ProjectSlugStr,
    ) -> AppResult<Vec<ProjectSnapshot>> {
        handle_project_permission_request(
            project_slug,
            Permission::Read, // Reading snapshots requires read permission
            None,             // No CSRF needed for read
            |_, pool, project_slug_obj| async move {
                // Fetch snapshots ordered by creation date, newest first
                Ok(sqlx::query_as!(
                    ProjectSnapshot,
                    "SELECT id, project_id, name,snapshot_name, description, created_at
                     FROM projects_snapshots
                     WHERE project_id = $1
                     ORDER BY created_at DESC", // Enforce limit
                    project_slug_obj.id
                )
                .fetch_all(&pool)
                .await?)
            },
        )
        .await
    }

    #[server]
    pub async fn create_project_snapshot(
        csrf: String,
        project_slug: ProjectSlugStr,
        name: String,
        description: Option<String>,
    ) -> AppResult<()> {
        // Basic validation (more could be added)
        if name.trim().is_empty() || name.len() > 100 {
            return Err(AppError::Custom("Invalid snapshot name.".to_string()));
        }
        if let Some(desc) = &description {
            if desc.len() > 500 {
                return Err(AppError::Custom("Description too long.".to_string()));
            }
        }

        handle_project_permission_request(
            project_slug,
            Permission::Write, // Creating requires write permission
            Some(csrf),
            |_, pool, project_slug| async move {

                ssr::ensure_not_max_snapshots(&pool, project_slug.clone(), 20).await?;
                let snapshot_name = format!("{}_snapshot_{}",project_slug, chrono::Utc::now().format("%Y_%m_%d_%H_%M_%S"));
                let _ = sqlx::query!(
                     r#"
                     INSERT INTO projects_snapshots (project_id, name,snapshot_name, description, created_at)
                     VALUES ($1, $2, $3,$4, NOW() at time zone 'utc')
                     RETURNING id
                     "#,
                     project_slug.id,
                     name,
                    snapshot_name,
                     description
                 )
                    .fetch_one(&pool)
                    .await?;

                request_server_project_action(
                    project_slug.clone(),
                    ServerProjectSnapshotAction::Create { snapshot_name }.into()
                ).await?;

                Ok(())
            },
        )
            .await
    }

    #[cfg(feature = "ssr")]
    pub mod ssr {
        use crate::{AppError, AppResult};
        use common::Slug;

        pub async fn ensure_not_max_snapshots(
            pool: &sqlx::PgPool,
            project_slug: Slug,
            max_snapshots: i64,
        ) -> AppResult<()> {
            // Check if the project has more than 20 snapshots
            let count_result = sqlx::query!(
                "SELECT COUNT(*) as count FROM projects_snapshots WHERE project_id = $1",
                project_slug.id
            )
            .fetch_one(pool)
            .await?;

            let count: i64 = count_result.count.unwrap_or(0);
            if count > max_snapshots {
                Err(AppError::ToMuchSnapshots)
            } else {
                Ok(())
            }
        }
    }

    #[server]
    pub async fn unset_active_project_snapshot(
        csrf: String,
        project_slug: ProjectSlugStr,
    ) -> AppResult<()> {
        handle_project_permission_request(
            project_slug,
            Permission::Owner, // Unsetting requires owner permission
            Some(csrf),
            |_, pool, project_slug| async move {
                let active_snapshot = sqlx::query!(
                    "SELECT active_snapshot_id FROM projects WHERE id = $1",
                    project_slug.id
                )
                .fetch_one(&pool)
                .await?;
                if active_snapshot.active_snapshot_id.is_none() {
                    return Err(AppError::NoActiveSnapshot);
                }
                sqlx::query!(
                    "UPDATE projects SET active_snapshot_id = NULL WHERE id = $1",
                    project_slug.id
                )
                .execute(&pool)
                .await?;

                request_server_project_action(
                    project_slug.clone(),
                    ServerProjectSnapshotAction::UnmountProd.into(),
                )
                .await?;
                request_hosting_action(project_slug, HostingAction::StopServingProject).await?;

                Ok(())
            },
        )
        .await
    }

    #[server]
    pub async fn delete_project_snapshot(
        csrf: String,
        project_slug: ProjectSlugStr,
        snapshot_id: i64,
    ) -> AppResult<()> {
        handle_project_permission_request(
            project_slug,
            Permission::Owner, // Deleting requires owner permission
            Some(csrf),
            |_, pool, project_slug_obj| async move {
                let active_snapshot = sqlx::query!(
                     "SELECT active_snapshot_id FROM projects WHERE id = $1",
                     project_slug_obj.id
                 )
                    .fetch_one(&pool)
                    .await?;
                if active_snapshot.active_snapshot_id == Some(snapshot_id) {
                    return Err(AppError::CantDeleteActiveSnapshot);
                }
                let snapshot = sqlx::query!(
                     "DELETE FROM projects_snapshots WHERE id = $1 AND project_id = $2 returning snapshot_name",
                     snapshot_id,
                     project_slug_obj.id
                 )
                    .fetch_optional(&pool)
                    .await?;
                if let Some(snapshot) = snapshot{
                    request_server_project_action(project_slug_obj.clone(), ServerProjectSnapshotAction::Delete { snapshot_name: snapshot.snapshot_name }.into()).await?;
                }

                Ok(())
            },
        )
            .await
    }

    #[server]
    pub async fn set_active_project_snapshot(
        csrf: String,
        project_slug: ProjectSlugStr,
        snapshot_id: i64,
    ) -> AppResult<()> {
        handle_project_permission_request(
            project_slug,
            Permission::Owner, // Setting active requires owner
            Some(csrf),
            |_, pool, project_slug| async move {
                // 1. Verify the snapshot exists for this project

                let snapshot = sqlx::query!(
                     "SELECT id,snapshot_name FROM projects_snapshots WHERE id = $1 AND project_id = $2",
                     snapshot_id,
                     project_slug.id
                 )
                    .fetch_optional(&pool)
                    .await?;
                if snapshot.is_none() {
                    return Err(AppError::Custom("Snapshot not found for this project.".to_string()));
                }
                let snapshot = snapshot.unwrap();
                let active_snapshot_id = sqlx::query!(
                     "SELECT active_snapshot_id FROM projects WHERE id = $1",
                     project_slug.id
                 )
                    .fetch_one(&pool)
                    .await?;
                if let Some(active_snapshot_id) = active_snapshot_id.active_snapshot_id {
                    if active_snapshot_id == snapshot.id {
                        return Err(AppError::Custom("Snapshot is already active.".to_string()));
                    }
                    request_server_project_action(project_slug.clone(), ServerProjectSnapshotAction::UnmountProd.into()).await?;
                }


                sqlx::query!(
                     "UPDATE projects SET active_snapshot_id = $1 WHERE id = $2",
                     snapshot_id,
                     project_slug.id
                 )
                    .execute(&pool)
                    .await?;

                request_server_project_action(project_slug.clone(), ServerProjectSnapshotAction::MountSnapshotProd { snapshot_name: snapshot.snapshot_name }.into()).await?;
                request_hosting_action(project_slug.clone(), HostingAction::ServeReloadProject).await?;

                Ok(())
            },
        )
            .await
    }
}
