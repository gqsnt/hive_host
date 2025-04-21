pub mod file_content_view;
pub mod project_files_sidebar;

use crate::api::get_action_server_project_action;
use crate::app::pages::user::projects::project::project_files::file_content_view::FileContentView;
use crate::app::pages::user::projects::project::project_files::project_files_sidebar::ProjectFilesSidebar;
use crate::app::pages::user::projects::project::MemoProjectParams;
use crate::app::IntoView;
use crate::app::ServerFnError;

use common::server_project_action::io_action::dir_action::{DirAction};
use common::server_project_action::ServerProjectActionResponse;
use leptos::either::Either;

use leptos::prelude::ElementChild;

use leptos::prelude::{
    expect_context, signal, ClassAttribute, CollectView, Get, IntoMaybeErased, OnAttribute,
};
use leptos::prelude::{Callback, Signal};
use leptos::server::Resource;
use leptos::{component, view};

#[component]
pub fn ProjectFiles() -> impl IntoView {
    let params: MemoProjectParams = expect_context();
    let slug = Signal::derive(move || params.get().unwrap().project_slug.clone());

    let (current_path, set_current_path) = signal(".".to_string());
    let (selected_file, set_selected_file) = signal::<Option<String>>(None);

    let server_project_action = get_action_server_project_action();

    let file_list_resource = Resource::new(
        move || {
            (
                current_path.get(),
                slug(),
                server_project_action.version().get(),
            )
        },
        |(path, slug, _)| async {
            match crate::api::get_action_server_project_action_inner(
                slug,
                DirAction::Ls { path }.into(),
                None,None
            )
            .await
            {
                Ok(ServerProjectActionResponse::Ls(files)) => Ok(files.inner),
                _ => Err(ServerFnError::new("Invalid response")),
            }
        },
    );

    let go_up_one_level = move |_| {
        let current = current_path.get();
        if current != "." {
            if let Some(last_slash_idx) = current.rfind('/') {
                if last_slash_idx == 1 {
                    set_current_path(".".to_string());
                } else {
                    set_current_path(current[..last_slash_idx].to_string());
                }
            }
        }
    };

    let breadcrumbs = move || {
        let path = current_path.get();
        let mut segments = vec![("Root".to_string(), ".".to_string())];
        if path != "." {
            let mut accumulated_path = String::from(".");
            let relative_path = path.trim_start_matches("./");
            for segment in relative_path.split('/') {
                if !segment.is_empty() {
                    accumulated_path.push('/');
                    accumulated_path.push_str(segment);
                    segments.push((segment.to_string(), accumulated_path.clone()));
                }
            }
        }
        segments
    };

    let handle_select_file = Callback::new(move |file_path: String| {
        set_selected_file(Some(file_path));
    });

    let handle_navigate_dir = Callback::new(move |dir_path: String| {
        set_current_path(dir_path);
        set_selected_file(None);
    });

    view! {
        <div class="flex flex-col h-full">

            <div class="flex-shrink-0 p-4 border-b border-white/10">
                <nav class="flex items-center space-x-1 text-sm text-gray-400 flex-wrap">
                    {move || {
                        breadcrumbs()
                            .into_iter()
                            .enumerate()
                            .map(|(i, (name, target_path))| {
                                let is_last = i == breadcrumbs().len() - 1;
                                view! {
                                    <span class="flex items-center">
                                        {(i > 0).then(|| view! { <span class="mx-1">/</span> })}
                                        {if is_last {
                                            Either::Left(
                                                view! {
                                                    <span class="font-medium text-white whitespace-nowrap">
                                                        {name}
                                                    </span>
                                                },
                                            )
                                        } else {
                                            Either::Right(
                                                view! {
                                                    <button
                                                        class="hover:text-white hover:underline whitespace-nowrap"
                                                        on:click=move |_| set_current_path(target_path.clone())
                                                    >
                                                        {name}
                                                    </button>
                                                },
                                            )
                                        }}
                                    </span>
                                }
                            })
                            .collect_view()
                    }}
                </nav>
            </div>

            <div class="flex flex-grow overflow-hidden">
                <div class="w-64 md:w-80 flex-shrink-0 border-r border-white/10 overflow-y-auto">
                    <ProjectFilesSidebar
                        file_list_resource=file_list_resource
                        current_path=current_path.into()
                        slug=slug
                        on_go_up=Callback::new(move |_| go_up_one_level(()))
                        on_navigate_dir=handle_navigate_dir
                        on_select_file=handle_select_file
                        server_project_action=server_project_action
                    />
                </div>
                <div class="flex-grow overflow-y-auto p-4 md:p-6 lg:p-8">
                    <FileContentView selected_file=selected_file.into() slug=slug />
                </div>
            </div>
        </div>
    }
}