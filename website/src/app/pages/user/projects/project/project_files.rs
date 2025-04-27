use leptos::prelude::{Effect, IntoAnyAttribute};
use leptos::prelude::AddAnyAttr;
pub mod file_content_view;
pub mod project_files_sidebar;

use leptos_router::params::Params;
use crate::api::get_action_server_project_action;
use crate::app::pages::user::projects::project::project_files::file_content_view::FileContentView;
use crate::app::pages::user::projects::project::project_files::project_files_sidebar::ProjectFilesSidebar;
use crate::app::pages::user::projects::project::{ ProjectSlugSignal};
use crate::app::IntoView;

use common::server_project_action::io_action::dir_action::DirAction;
use common::server_project_action::ServerProjectActionResponse;
use leptos::either::Either;

use leptos::prelude::{ElementChild, Memo, Read, Suspend, Transition};

use crate::app::pages::{GlobalState, GlobalStateStoreFields};
use leptos::prelude::{
    expect_context, signal, ClassAttribute, CollectView, Get, IntoMaybeErased,
};
use leptos::prelude::{Callback, Signal};
use leptos::server::Resource;
use leptos::{component, view, Params};
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params};
use leptos_router::params::ParamsError;
use reactive_stores::Store;

#[derive(Params, Clone, Debug, PartialEq)]
pub struct ProjectFilesParams {
    pub path: String,
}

pub type MemoProjectFilesParams = Memo<Result<ProjectFilesParams, ParamsError>>;

#[component]
pub fn ProjectFiles() -> impl IntoView {
    let params: MemoProjectFilesParams = use_params::<ProjectFilesParams>();
    let current_path =  Signal::derive(move || {
        params.read()
            .as_ref()
            .ok()
            .map(|p|{
                let p = p.path.clone();
                if p.is_empty(){
                    return "root/".to_string();
                }
                let end_with_slash = p.ends_with("/");
                if end_with_slash {
                     p
                }else{
                    let mut p = p.clone();
                    p.push('/');
                     p
                }
            })
            .unwrap_or_else(|| "root/".to_string())
    });
    
    let global_state:Store<GlobalState> = expect_context();

    let project_slug_signal:Signal<ProjectSlugSignal> = expect_context();
    let slug = Signal::derive(move ||
        project_slug_signal.read().0.clone());

    let csrf_signal =  Signal::derive(move || {
        global_state.csrf().get()
    });
    
    
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
        |(path,slug, _)| {
            crate::api::get_action_server_project_action_inner(
                slug,
                DirAction::Ls { path }.into(),
                None,
                None,
            )
        },
    );

    let breadcrumbs = move || {
        let path = current_path.get();
        let mut segments = vec![("Root".to_string(), "root/".to_string())];
        if path != "root/" {
            let mut accumulated_path = String::from("root");
            let relative_path = path.trim_start_matches("root/");
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
    Effect::new(move |_|{
        let _ = current_path.get();
        set_selected_file(None);
    });

    let handle_select_file = Callback::new(move |file_path: String| {
        set_selected_file(Some(file_path));
    });
    // 
    // let handle_navigate_dir = Callback::new(move |dir_path: String| {
    //     set_current_path(dir_path);
    //     set_selected_file(None);
    // });
    
    

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
                                                    <A
                                                        attr:class="hover:text-white hover:underline whitespace-nowrap"
                                                        href=move || {
                                                            format!("/user/projects/{}/files/{}", slug(), target_path)
                                                        }
                                                    >
                                                        {name}
                                                    </A>
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

                <Transition fallback=move || {
                    view! { Loading... }
                }>
                    {move || {
                        Suspend::new(async move {
                            let (file_list, _) = signal(
                                file_list_resource
                                    .get()
                                    .and_then(|r| {
                                        r.ok()
                                            .and_then(|r| match r {
                                                ServerProjectActionResponse::Ls(inner) => Some(inner.inner),
                                                _ => None,
                                            })
                                    }),
                            );
                            Effect::new(move |_| {
                                if file_list.read().as_ref().is_none() {
                                    let navigate = use_navigate();
                                    navigate(
                                        &format!("/user/projects/{}/files/root/", slug()),
                                        Default::default(),
                                    );
                                }
                            });

                            view! {
                                <div class="w-64 md:w-80 flex-shrink-0 border-r border-white/10 overflow-y-auto">
                                    <ProjectFilesSidebar
                                        csrf_signal
                                        file_list=file_list
                                        current_path=current_path
                                        slug=slug
                                        // on_go_up=handle_on_go_up
                                        // on_navigate_dir=handle_navigate_dir
                                        on_select_file=handle_select_file
                                        server_project_action=server_project_action
                                    />

                                </div>
                            }
                        })
                    }}
                </Transition>
                <div class="flex-grow overflow-y-auto p-4 md:p-6 lg:p-8">
                    <FileContentView csrf_signal selected_file=selected_file.into() slug=slug />
                </div>
            </div>
        </div>
    }
}

pub mod server_fns {
    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
    }}
}
