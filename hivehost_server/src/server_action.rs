use crate::{ServerResult, TarpcHelperClient};
use common::helper_command::{HelperCommand, HelperResponse};
use common::server_action::permission::Permission;
use common::server_action::user_action::{ServerUserAction, ServerUserResponse};
use common::{
    get_project_dev_path, get_user_path, get_user_project_path, get_user_projects_path,
    ProjectSlugStr, UserSlugStr,
};
use std::path::Path;
use tracing::info;

pub async fn handle_user_action(
    server_helper: TarpcHelperClient,
    action: ServerUserAction,
) -> ServerResult<ServerUserResponse> {
    info!("Server action: {:?}", action);
     let r = match action {
        ServerUserAction::Create { user_slug } => create_user(server_helper, user_slug.to_string()).await?,
        ServerUserAction::Delete { user_slug } => remove_user(server_helper, user_slug.to_string()).await?,
        ServerUserAction::AddProject {
            user_slug,
            project_slug,
        } => create_project(server_helper, user_slug.to_string(), project_slug.to_string()).await?,
        ServerUserAction::RemoveProject {
            user_slugs,
            project_slug,
        } => {
            let mut helper_commands = user_slugs
                .iter()
                .flat_map(|user_slug| {
                    remove_user_from_project_commands(
                        user_slug.to_string(),
                        project_slug.to_string(),
                    )
                })
                .collect::<Vec<HelperCommand>>();
            helper_commands.push(HelperCommand::DeleteProject { project_slug:project_slug.to_string() });
            server_helper.execute(helper_commands).await?
        }
    };
    Ok(ServerUserResponse::Helper(r))
}

pub async fn create_user(
    server_helper:TarpcHelperClient,
    user_slug: UserSlugStr,
) -> ServerResult<HelperResponse> {
    let user_path = get_user_path(&user_slug);
    let user_projects_path = get_user_projects_path(&user_slug);
    Ok(server_helper.execute(vec![
        HelperCommand::CreateUser {
            user_slug,
            user_path: user_path.clone(),
            user_projects_path: user_projects_path.clone(),
        }
    ]).await?)
}

pub async fn create_project(
    server_helper:TarpcHelperClient,
    user_slug: UserSlugStr,
    project_slug: ProjectSlugStr,
) -> ServerResult<HelperResponse> {
    let project_slug_clone = project_slug.clone();
    let user_slug_clone = user_slug.clone();
    let dev_path = get_project_dev_path(&project_slug);
    let user_project_path = get_user_project_path(&user_slug, &project_slug);
    let dev_path_clone = dev_path.clone();

    Ok(server_helper.execute(vec![
        HelperCommand::CreateProject {
            project_slug: project_slug_clone,
            service_user: user_slug_clone,
        },
        HelperCommand::BindMountUserProject {
            source_path: dev_path_clone,
            target_path: user_project_path.clone(),
        }
    ]).await?)

}

pub async fn add_user_to_project(
    server_helper:TarpcHelperClient,
    user_slug: UserSlugStr,
    project_slug: ProjectSlugStr,
    permission: Permission,
) -> ServerResult<HelperResponse> {
    let project_path = get_project_dev_path(&project_slug);
    let project_path_clone = project_path.clone();
    let user_project_path = get_user_project_path(&user_slug, &project_slug);
    Ok(server_helper.execute(vec![
        HelperCommand::SetAcl {
            path: project_path_clone,
            user_slug,
            is_read_only: permission.is_read_only(),
        },
        HelperCommand::BindMountUserProject {
            source_path: project_path,
            target_path: user_project_path,
        }
    ]).await?)
}

pub async  fn update_user_in_project(
    server_helper:TarpcHelperClient,
    user_slug: UserSlugStr,
    project_slug: ProjectSlugStr,
    permission: Permission,
) -> ServerResult<HelperResponse>{
    Ok(server_helper.execute(vec![
        HelperCommand::SetAcl {
            path: get_project_dev_path(&project_slug),
            user_slug,
            is_read_only: permission.is_read_only(),
        }
    ]).await?)
}

pub fn remove_user_from_project_commands(
    user_slug: UserSlugStr,
    project_slug: ProjectSlugStr,
) ->Vec<HelperCommand> {
    let proj_path = get_project_dev_path(&project_slug);
    let user_project_path = get_user_project_path(&user_slug, &project_slug);
    vec![
        HelperCommand::RemoveAcl {
            path: proj_path.clone(),
            user_slug: user_slug.clone(),
        },
        HelperCommand::UnmountUserProject {
            target_path: user_project_path.clone(),
        },
    ]
}


pub async fn remove_user(
    server_helper:TarpcHelperClient,
    user_slug: UserSlugStr,
) -> ServerResult<HelperResponse> {
    let user_projects_path = get_user_projects_path(&user_slug);
    let path = Path::new(&user_projects_path);
    let mut commands = vec![];
    if path.exists() {
        let mut read_dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let submount = entry.path();
            if submount.is_dir() {
                commands.push(HelperCommand::UnmountUserProject {
                    target_path: submount.to_string_lossy().to_string(),
                });
            }
        }
    }
    let user_slug_clone = user_slug.clone();
    commands.push(HelperCommand::DeleteUser {
        user_slug: user_slug_clone,
        user_path: get_user_path(&user_slug),
    });
    Ok(server_helper.execute(commands).await?)
}
