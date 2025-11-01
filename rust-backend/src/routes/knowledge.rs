use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use tracing as log;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::{AuthMiddleware, AuthUser};
use crate::models::knowledge::{KnowledgeFilesResponse, KnowledgeResponse, KnowledgeUserResponse};
use crate::services::file::FileService;
use crate::services::group::GroupService;
use crate::services::knowledge::KnowledgeService;
use crate::services::user::UserService;
use crate::utils::misc::{has_access, has_permission};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct KnowledgeForm {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub access_control: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct KnowledgeFileIdForm {
    pub file_id: String,
}

pub fn create_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("")
            .wrap(AuthMiddleware)
            .route(web::get().to(get_knowledge_bases)),
    )
    .service(
        web::resource("/")
            .wrap(AuthMiddleware)
            .route(web::get().to(get_knowledge_bases)),
    )
    .service(
        web::resource("/list")
            .wrap(AuthMiddleware)
            .route(web::get().to(get_knowledge_list)),
    )
    .service(
        web::resource("/create")
            .wrap(AuthMiddleware)
            .route(web::post().to(create_knowledge)),
    )
    .service(
        web::resource("/reindex")
            .wrap(AuthMiddleware)
            .route(web::post().to(reindex_all_knowledge)),
    )
    .service(
        web::resource("/{id}")
            .wrap(AuthMiddleware)
            .route(web::get().to(get_knowledge_by_id)),
    )
    .service(
        web::resource("/{id}/update")
            .wrap(AuthMiddleware)
            .route(web::post().to(update_knowledge)),
    )
    .service(
        web::resource("/{id}/delete")
            .wrap(AuthMiddleware)
            .route(web::delete().to(delete_knowledge_by_id)),
    )
    .service(
        web::resource("/{id}/file/add")
            .wrap(AuthMiddleware)
            .route(web::post().to(add_file_to_knowledge)),
    )
    .service(
        web::resource("/{id}/file/update")
            .wrap(AuthMiddleware)
            .route(web::post().to(update_file_in_knowledge)),
    )
    .service(
        web::resource("/{id}/file/remove")
            .wrap(AuthMiddleware)
            .route(web::post().to(remove_file_from_knowledge)),
    )
    .service(
        web::resource("/{id}/reset")
            .wrap(AuthMiddleware)
            .route(web::post().to(reset_knowledge)),
    )
    .service(
        web::resource("/{id}/files/batch/add")
            .wrap(AuthMiddleware)
            .route(web::post().to(add_files_batch)),
    );
}

// GET / - Get knowledge bases with read access
async fn get_knowledge_bases(
    state: web::Data<AppState>,
    auth_user: AuthUser,
) -> AppResult<HttpResponse> {
    let knowledge_service = KnowledgeService::new(&state.db);
    let user_service = UserService::new(&state.db);
    let file_service = FileService::new(&state.db);

    let config = state.config.read().unwrap();
    let bypass_admin_access = config.bypass_admin_access_control.unwrap_or(false);
    drop(config);

    let knowledge_bases = if auth_user.user.role == "admin" && bypass_admin_access {
        knowledge_service.get_all_knowledge().await?
    } else {
        let group_service = GroupService::new(&state.db);
        let groups = group_service
            .get_groups_by_member_id(&auth_user.user.id)
            .await?;
        let user_group_ids: HashSet<String> = groups.into_iter().map(|g| g.id).collect();

        let all_knowledge = knowledge_service.get_all_knowledge().await?;
        all_knowledge
            .into_iter()
            .filter(|k| {
                k.user_id == auth_user.user.id
                    || has_access(
                        &auth_user.user.id,
                        "read",
                        &k.access_control,
                        &user_group_ids,
                    )
            })
            .collect()
    };

    // Get unique user IDs
    let user_ids: HashSet<String> = knowledge_bases.iter().map(|k| k.user_id.clone()).collect();

    // Fetch users
    let mut users_map: HashMap<String, serde_json::Value> = HashMap::new();
    for user_id in user_ids {
        if let Ok(Some(user)) = user_service.get_user_by_id(&user_id).await {
            users_map.insert(
                user_id.clone(),
                json!({
                    "id": user.id,
                    "name": user.name,
                    "email": user.email,
                    "role": user.role,
                    "profile_image_url": user.profile_image_url,
                }),
            );
        }
    }

    // Get files for each knowledge base
    let mut responses = Vec::new();
    for mut knowledge in knowledge_bases {
        let mut files = Vec::new();
        if let Some(data) = &knowledge.data {
            if let Some(file_ids) = data.get("file_ids").and_then(|v| v.as_array()) {
                let file_id_strings: Vec<String> = file_ids
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();

                // Get file metadatas
                if let Ok(file_metadatas) = file_service
                    .get_file_metadatas_by_ids(&file_id_strings)
                    .await
                {
                    files = file_metadatas;

                    // Check if all files exist - clean up missing files
                    if files.len() != file_id_strings.len() {
                        let existing_ids: Vec<String> = files
                            .iter()
                            .filter_map(|f| {
                                f.get("id").and_then(|id| id.as_str().map(String::from))
                            })
                            .collect();

                        let missing_files: Vec<_> = file_id_strings
                            .iter()
                            .filter(|id| !existing_ids.contains(id))
                            .collect();

                        if !missing_files.is_empty() {
                            // Update knowledge data to remove missing files
                            let mut data = knowledge.data.clone().unwrap_or_else(|| json!({}));
                            data["file_ids"] = json!(existing_ids);

                            // Update in database
                            if let Ok(updated) = knowledge_service
                                .update_knowledge_data(&knowledge.id, data)
                                .await
                            {
                                knowledge = updated;
                            }

                            // Update files list with only existing files
                            if let Ok(updated_metadatas) =
                                file_service.get_file_metadatas_by_ids(&existing_ids).await
                            {
                                files = updated_metadatas;
                            }
                        }
                    }
                }
            }
        }

        let user = users_map.get(&knowledge.user_id).cloned();
        responses.push(KnowledgeUserResponse::from_knowledge_and_user(
            knowledge,
            user,
            Some(files),
        ));
    }

    Ok(HttpResponse::Ok().json(responses))
}

// GET /list - Get knowledge list (with write access)
async fn get_knowledge_list(
    state: web::Data<AppState>,
    auth_user: AuthUser,
) -> AppResult<HttpResponse> {
    let knowledge_service = KnowledgeService::new(&state.db);
    let user_service = UserService::new(&state.db);
    let file_service = FileService::new(&state.db);

    let config = state.config.read().unwrap();
    let bypass_admin_access = config.bypass_admin_access_control.unwrap_or(false);
    drop(config);

    let knowledge_bases = if auth_user.user.role == "admin" && bypass_admin_access {
        knowledge_service.get_all_knowledge().await?
    } else {
        let group_service = GroupService::new(&state.db);
        let groups = group_service
            .get_groups_by_member_id(&auth_user.user.id)
            .await?;
        let user_group_ids: HashSet<String> = groups.into_iter().map(|g| g.id).collect();

        let all_knowledge = knowledge_service.get_all_knowledge().await?;
        all_knowledge
            .into_iter()
            .filter(|k| {
                k.user_id == auth_user.user.id
                    || has_access(
                        &auth_user.user.id,
                        "write",
                        &k.access_control,
                        &user_group_ids,
                    )
            })
            .collect()
    };

    // Get unique user IDs
    let user_ids: HashSet<String> = knowledge_bases.iter().map(|k| k.user_id.clone()).collect();

    // Fetch users
    let mut users_map: HashMap<String, serde_json::Value> = HashMap::new();
    for user_id in user_ids {
        if let Ok(Some(user)) = user_service.get_user_by_id(&user_id).await {
            users_map.insert(
                user_id.clone(),
                json!({
                    "id": user.id,
                    "name": user.name,
                    "email": user.email,
                    "role": user.role,
                    "profile_image_url": user.profile_image_url,
                }),
            );
        }
    }

    // Get files for each knowledge base
    let mut responses = Vec::new();
    for mut knowledge in knowledge_bases {
        let mut files = Vec::new();
        if let Some(data) = &knowledge.data {
            if let Some(file_ids) = data.get("file_ids").and_then(|v| v.as_array()) {
                let file_id_strings: Vec<String> = file_ids
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();

                // Get file metadatas
                if let Ok(file_metadatas) = file_service
                    .get_file_metadatas_by_ids(&file_id_strings)
                    .await
                {
                    files = file_metadatas;

                    // Check if all files exist - clean up missing files
                    if files.len() != file_id_strings.len() {
                        let existing_ids: Vec<String> = files
                            .iter()
                            .filter_map(|f| {
                                f.get("id").and_then(|id| id.as_str().map(String::from))
                            })
                            .collect();

                        let missing_files: Vec<_> = file_id_strings
                            .iter()
                            .filter(|id| !existing_ids.contains(id))
                            .collect();

                        if !missing_files.is_empty() {
                            // Update knowledge data to remove missing files
                            let mut data = knowledge.data.clone().unwrap_or_else(|| json!({}));
                            data["file_ids"] = json!(existing_ids);

                            // Update in database
                            if let Ok(updated) = knowledge_service
                                .update_knowledge_data(&knowledge.id, data)
                                .await
                            {
                                knowledge = updated;
                            }

                            // Update files list with only existing files
                            if let Ok(updated_metadatas) =
                                file_service.get_file_metadatas_by_ids(&existing_ids).await
                            {
                                files = updated_metadatas;
                            }
                        }
                    }
                }
            }
        }

        let user = users_map.get(&knowledge.user_id).cloned();
        responses.push(KnowledgeUserResponse::from_knowledge_and_user(
            knowledge,
            user,
            Some(files),
        ));
    }

    Ok(HttpResponse::Ok().json(responses))
}

// POST /create - Create new knowledge base
async fn create_knowledge(
    state: web::Data<AppState>,
    auth_user: AuthUser,
    form: web::Json<KnowledgeForm>,
) -> AppResult<HttpResponse> {
    // Check workspace.knowledge permission
    if auth_user.user.role != "admin" {
        let config = state.config.read().unwrap();
        let user_permissions = config.user_permissions.clone();
        drop(config);

        if !has_permission(&auth_user.user.id, "workspace.knowledge", &user_permissions) {
            return Err(AppError::Unauthorized("Unauthorized".to_string()));
        }
    }

    // Check if user can share publicly
    let mut access_control = form.access_control.clone();
    if auth_user.user.role != "admin" && access_control.is_none() {
        let config = state.config.read().unwrap();
        let user_permissions = config.user_permissions.clone();
        drop(config);

        if !has_permission(
            &auth_user.user.id,
            "sharing.public_knowledge",
            &user_permissions,
        ) {
            access_control = Some(json!({}));
        }
    }

    let knowledge_service = KnowledgeService::new(&state.db);
    let knowledge_id = Uuid::new_v4().to_string();

    let mut data = form.data.clone().unwrap_or_else(|| json!({}));
    if data.get("file_ids").is_none() {
        data["file_ids"] = json!([]);
    }

    let knowledge = knowledge_service
        .create_knowledge_with_access_control(
            &knowledge_id,
            &auth_user.user.id,
            &form.name,
            Some(&form.description),
            Some(data),
            access_control,
        )
        .await?;

    Ok(HttpResponse::Ok().json(KnowledgeResponse::from(knowledge)))
}

// GET /{id} - Get knowledge by ID
async fn get_knowledge_by_id(
    state: web::Data<AppState>,
    auth_user: AuthUser,
    knowledge_id: web::Path<String>,
) -> AppResult<HttpResponse> {
    let knowledge_service = KnowledgeService::new(&state.db);
    let file_service = FileService::new(&state.db);

    let knowledge = knowledge_service
        .get_knowledge_by_id(&knowledge_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Knowledge not found".to_string()))?;

    // Check access: owner, admin, or has read access
    if auth_user.user.role != "admin" && knowledge.user_id != auth_user.user.id {
        let group_service = GroupService::new(&state.db);
        let groups = group_service
            .get_groups_by_member_id(&auth_user.user.id)
            .await?;
        let user_group_ids: HashSet<String> = groups.into_iter().map(|g| g.id).collect();

        if !has_access(
            &auth_user.user.id,
            "read",
            &knowledge.access_control,
            &user_group_ids,
        ) {
            return Err(AppError::Unauthorized("Not found".to_string()));
        }
    }

    // Get files
    let mut files = Vec::new();
    if let Some(data) = &knowledge.data {
        if let Some(file_ids) = data.get("file_ids").and_then(|v| v.as_array()) {
            let file_id_strings: Vec<String> = file_ids
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();

            for file_id in file_id_strings {
                if let Ok(Some(file)) = file_service.get_file_by_id(&file_id).await {
                    files.push(json!({
                        "id": file.id,
                        "filename": file.filename,
                        "meta": file.meta,
                        "created_at": file.created_at,
                        "updated_at": file.updated_at,
                    }));
                }
            }
        }
    }

    let response = KnowledgeFilesResponse::from_knowledge_and_files(knowledge, files);
    Ok(HttpResponse::Ok().json(response))
}

// POST /{id}/update - Update knowledge
async fn update_knowledge(
    state: web::Data<AppState>,
    auth_user: AuthUser,
    knowledge_id: web::Path<String>,
    form: web::Json<KnowledgeForm>,
) -> AppResult<HttpResponse> {
    let knowledge_service = KnowledgeService::new(&state.db);
    let file_service = FileService::new(&state.db);

    let knowledge = knowledge_service
        .get_knowledge_by_id(&knowledge_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Knowledge not found".to_string()))?;

    // Check write access
    if knowledge.user_id != auth_user.user.id && auth_user.user.role != "admin" {
        let group_service = GroupService::new(&state.db);
        let groups = group_service
            .get_groups_by_member_id(&auth_user.user.id)
            .await?;
        let user_group_ids: HashSet<String> = groups.into_iter().map(|g| g.id).collect();

        if !has_access(
            &auth_user.user.id,
            "write",
            &knowledge.access_control,
            &user_group_ids,
        ) {
            return Err(AppError::Forbidden("Access prohibited".to_string()));
        }
    }

    // Check if user can share publicly
    let mut access_control = form.access_control.clone();
    if auth_user.user.role != "admin" && access_control.is_none() {
        let config = state.config.read().unwrap();
        let user_permissions = config.user_permissions.clone();
        drop(config);

        if !has_permission(
            &auth_user.user.id,
            "sharing.public_knowledge",
            &user_permissions,
        ) {
            access_control = Some(json!({}));
        }
    }

    let updated = knowledge_service
        .update_knowledge_full(
            &knowledge_id,
            Some(&form.name),
            Some(&form.description),
            form.data.clone(),
            access_control,
        )
        .await?;

    // Get files
    let mut files = Vec::new();
    if let Some(data) = &updated.data {
        if let Some(file_ids) = data.get("file_ids").and_then(|v| v.as_array()) {
            let file_id_strings: Vec<String> = file_ids
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();

            for file_id in file_id_strings {
                if let Ok(Some(file)) = file_service.get_file_by_id(&file_id).await {
                    files.push(json!({
                        "id": file.id,
                        "filename": file.filename,
                        "meta": file.meta,
                        "created_at": file.created_at,
                        "updated_at": file.updated_at,
                    }));
                }
            }
        }
    }

    let response = KnowledgeFilesResponse::from_knowledge_and_files(updated, files);
    Ok(HttpResponse::Ok().json(response))
}

// DELETE /{id}/delete - Delete knowledge by ID
async fn delete_knowledge_by_id(
    state: web::Data<AppState>,
    auth_user: AuthUser,
    knowledge_id: web::Path<String>,
) -> AppResult<HttpResponse> {
    let knowledge_service = KnowledgeService::new(&state.db);
    let model_service = crate::services::model::ModelService::new(&state.db);

    let knowledge = knowledge_service
        .get_knowledge_by_id(&knowledge_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Knowledge not found".to_string()))?;

    // Check write access
    if knowledge.user_id != auth_user.user.id && auth_user.user.role != "admin" {
        let group_service = GroupService::new(&state.db);
        let groups = group_service
            .get_groups_by_member_id(&auth_user.user.id)
            .await?;
        let user_group_ids: HashSet<String> = groups.into_iter().map(|g| g.id).collect();

        if !has_access(
            &auth_user.user.id,
            "write",
            &knowledge.access_control,
            &user_group_ids,
        ) {
            return Err(AppError::Forbidden("Access prohibited".to_string()));
        }
    }

    log::info!(
        "Deleting knowledge base: {} (name: {})",
        knowledge_id.as_str(),
        knowledge.name
    );

    // Get all models and update those that reference this knowledge base
    if let Ok(models) = model_service.get_all_models().await {
        log::info!(
            "Found {} models to check for knowledge base {}",
            models.len(),
            knowledge_id.as_str()
        );

        for model in models {
            if let Some(meta) = &model.meta {
                if let Some(knowledge_list) = meta.get("knowledge").and_then(|k| k.as_array()) {
                    // Filter out the deleted knowledge base
                    let updated_knowledge: Vec<serde_json::Value> = knowledge_list
                        .iter()
                        .filter(|k| {
                            k.get("id")
                                .and_then(|id| id.as_str())
                                .map(|id| id != knowledge_id.as_str())
                                .unwrap_or(true)
                        })
                        .cloned()
                        .collect();

                    // If the knowledge list changed, update the model
                    if updated_knowledge.len() != knowledge_list.len() {
                        log::info!(
                            "Updating model {} to remove knowledge base {}",
                            model.id,
                            knowledge_id.as_str()
                        );

                        let mut updated_meta = meta.clone();
                        updated_meta["knowledge"] = json!(updated_knowledge);

                        let model_form = crate::models::model::ModelForm {
                            id: model.id.clone(),
                            base_model_id: model.base_model_id.clone(),
                            name: model.name.clone(),
                            params: model.params.clone(),
                            meta: updated_meta,
                            access_control: model.access_control.clone(),
                        };

                        if let Err(e) = model_service
                            .update_model_by_id(&model.id, model_form)
                            .await
                        {
                            log::error!("Failed to update model {}: {}", model.id, e);
                        }
                    }
                }
            }
        }
    }

    // TODO: Delete vector collection (requires vector DB integration)
    // Python equivalent:
    //   try:
    //       VECTOR_DB_CLIENT.delete_collection(collection_name=id)
    //   except Exception as e:
    //       log.debug(e)
    //       pass
    // This deletes the entire vector database collection for this knowledge base,
    // removing all embeddings for all files in the knowledge base

    knowledge_service.delete_knowledge(&knowledge_id).await?;

    Ok(HttpResponse::Ok().json(true))
}

// POST /{id}/file/add - Add file to knowledge
async fn add_file_to_knowledge(
    state: web::Data<AppState>,
    auth_user: AuthUser,
    knowledge_id: web::Path<String>,
    form: web::Json<KnowledgeFileIdForm>,
) -> AppResult<HttpResponse> {
    let knowledge_service = KnowledgeService::new(&state.db);
    let file_service = FileService::new(&state.db);

    let knowledge = knowledge_service
        .get_knowledge_by_id(&knowledge_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Knowledge not found".to_string()))?;

    // Check write access
    if knowledge.user_id != auth_user.user.id && auth_user.user.role != "admin" {
        let group_service = GroupService::new(&state.db);
        let groups = group_service
            .get_groups_by_member_id(&auth_user.user.id)
            .await?;
        let user_group_ids: HashSet<String> = groups.into_iter().map(|g| g.id).collect();

        if !has_access(
            &auth_user.user.id,
            "write",
            &knowledge.access_control,
            &user_group_ids,
        ) {
            return Err(AppError::Forbidden("Access prohibited".to_string()));
        }
    }

    // Check if file exists
    let file = file_service
        .get_file_by_id(&form.file_id)
        .await?
        .ok_or_else(|| AppError::NotFound("File not found".to_string()))?;

    // Check if file has been processed (has data)
    if file.data.is_none() {
        return Err(AppError::BadRequest("File not processed".to_string()));
    }

    // TODO: Process file and add to vector DB (requires vector DB integration)
    // This should call the retrieval service's process_file function with:
    // - file_id: form.file_id
    // - collection_name: knowledge_id (the knowledge base ID)
    // - user: auth_user
    // Python equivalent:
    //   process_file(
    //       request,
    //       ProcessFileForm(file_id=form.file_id, collection_name=id),
    //       user=user
    //   )
    // The process_file function will:
    // 1. Load the file content from storage
    // 2. Extract and chunk the text
    // 3. Generate embeddings for each chunk
    // 4. Store embeddings in the vector database collection named after the knowledge base ID

    // Add file ID to knowledge data
    let mut data = knowledge.data.clone().unwrap_or_else(|| json!({}));
    let mut file_ids = data
        .get("file_ids")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    if !file_ids.contains(&form.file_id) {
        file_ids.push(form.file_id.clone());
        data["file_ids"] = json!(file_ids);

        let updated = knowledge_service
            .update_knowledge_data(&knowledge_id, data)
            .await?;

        // Get files
        let mut files = Vec::new();
        if let Some(data) = &updated.data {
            if let Some(file_ids) = data.get("file_ids").and_then(|v| v.as_array()) {
                let file_id_strings: Vec<String> = file_ids
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();

                for file_id in file_id_strings {
                    if let Ok(Some(file)) = file_service.get_file_by_id(&file_id).await {
                        files.push(json!({
                            "id": file.id,
                            "filename": file.filename,
                            "meta": file.meta,
                            "created_at": file.created_at,
                            "updated_at": file.updated_at,
                        }));
                    }
                }
            }
        }

        let response = KnowledgeFilesResponse::from_knowledge_and_files(updated, files);
        return Ok(HttpResponse::Ok().json(response));
    }

    Err(AppError::BadRequest(
        "File already in knowledge base".to_string(),
    ))
}

// POST /{id}/file/update - Update file in knowledge
async fn update_file_in_knowledge(
    state: web::Data<AppState>,
    auth_user: AuthUser,
    knowledge_id: web::Path<String>,
    form: web::Json<KnowledgeFileIdForm>,
) -> AppResult<HttpResponse> {
    let knowledge_service = KnowledgeService::new(&state.db);
    let file_service = FileService::new(&state.db);

    let knowledge = knowledge_service
        .get_knowledge_by_id(&knowledge_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Knowledge not found".to_string()))?;

    // Check write access
    if knowledge.user_id != auth_user.user.id && auth_user.user.role != "admin" {
        let group_service = GroupService::new(&state.db);
        let groups = group_service
            .get_groups_by_member_id(&auth_user.user.id)
            .await?;
        let user_group_ids: HashSet<String> = groups.into_iter().map(|g| g.id).collect();

        if !has_access(
            &auth_user.user.id,
            "write",
            &knowledge.access_control,
            &user_group_ids,
        ) {
            return Err(AppError::Forbidden("Access prohibited".to_string()));
        }
    }

    // Check if file exists
    let _file = file_service
        .get_file_by_id(&form.file_id)
        .await?
        .ok_or_else(|| AppError::NotFound("File not found".to_string()))?;

    // TODO: Remove old content from vector DB and re-index (requires vector DB integration)
    // Python equivalent:
    //   VECTOR_DB_CLIENT.delete(
    //       collection_name=knowledge.id,
    //       filter={"file_id": form.file_id}
    //   )
    //   process_file(
    //       request,
    //       ProcessFileForm(file_id=form.file_id, collection_name=id),
    //       user=user
    //   )
    // This should:
    // 1. Delete all vector embeddings for this file from the knowledge base collection
    // 2. Re-process the file to generate fresh embeddings
    // 3. Store the new embeddings in the collection

    // Get files for response
    let mut files = Vec::new();
    if let Some(data) = &knowledge.data {
        if let Some(file_ids) = data.get("file_ids").and_then(|v| v.as_array()) {
            let file_id_strings: Vec<String> = file_ids
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();

            for file_id in file_id_strings {
                if let Ok(Some(file)) = file_service.get_file_by_id(&file_id).await {
                    files.push(json!({
                        "id": file.id,
                        "filename": file.filename,
                        "meta": file.meta,
                        "created_at": file.created_at,
                        "updated_at": file.updated_at,
                    }));
                }
            }
        }
    }

    let response = KnowledgeFilesResponse::from_knowledge_and_files(knowledge, files);
    Ok(HttpResponse::Ok().json(response))
}

// POST /{id}/file/remove - Remove file from knowledge
#[derive(Debug, Deserialize)]
struct RemoveFileQuery {
    delete_file: Option<bool>,
}

async fn remove_file_from_knowledge(
    state: web::Data<AppState>,
    auth_user: AuthUser,
    knowledge_id: web::Path<String>,
    form: web::Json<KnowledgeFileIdForm>,
    query: web::Query<RemoveFileQuery>,
) -> AppResult<HttpResponse> {
    let knowledge_service = KnowledgeService::new(&state.db);
    let file_service = FileService::new(&state.db);

    let knowledge = knowledge_service
        .get_knowledge_by_id(&knowledge_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Knowledge not found".to_string()))?;

    // Check write access
    if knowledge.user_id != auth_user.user.id && auth_user.user.role != "admin" {
        let group_service = GroupService::new(&state.db);
        let groups = group_service
            .get_groups_by_member_id(&auth_user.user.id)
            .await?;
        let user_group_ids: HashSet<String> = groups.into_iter().map(|g| g.id).collect();

        if !has_access(
            &auth_user.user.id,
            "write",
            &knowledge.access_control,
            &user_group_ids,
        ) {
            return Err(AppError::Forbidden("Access prohibited".to_string()));
        }
    }

    // TODO: Remove content from vector DB (requires vector DB integration)
    // Python equivalent:
    //   try:
    //       VECTOR_DB_CLIENT.delete(
    //           collection_name=knowledge.id,
    //           filter={"file_id": form.file_id}
    //       )
    //   except Exception as e:
    //       log.debug("This was most likely caused by bypassing embedding processing")
    //       pass
    // This removes all vector embeddings for this file from the knowledge base collection

    // Delete file from database if requested
    let delete_file = query.delete_file.unwrap_or(true);
    if delete_file {
        // TODO: Delete file's collection from vector database
        // Python equivalent:
        //   try:
        //       file_collection = f"file-{form.file_id}"
        //       if VECTOR_DB_CLIENT.has_collection(collection_name=file_collection):
        //           VECTOR_DB_CLIENT.delete_collection(collection_name=file_collection)
        //   except Exception as e:
        //       pass
        // This deletes the entire collection for this individual file
        file_service.delete_file(&form.file_id).await?;
    }

    // Remove file ID from knowledge data
    let mut data = knowledge.data.clone().unwrap_or_else(|| json!({}));
    let mut file_ids = data
        .get("file_ids")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    if let Some(pos) = file_ids.iter().position(|id| id == &form.file_id) {
        file_ids.remove(pos);
        data["file_ids"] = json!(file_ids);

        let updated = knowledge_service
            .update_knowledge_data(&knowledge_id, data)
            .await?;

        // Get files
        let mut files = Vec::new();
        if let Some(data) = &updated.data {
            if let Some(file_ids) = data.get("file_ids").and_then(|v| v.as_array()) {
                let file_id_strings: Vec<String> = file_ids
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();

                for file_id in file_id_strings {
                    if let Ok(Some(file)) = file_service.get_file_by_id(&file_id).await {
                        files.push(json!({
                            "id": file.id,
                            "filename": file.filename,
                            "meta": file.meta,
                            "created_at": file.created_at,
                            "updated_at": file.updated_at,
                        }));
                    }
                }
            }
        }

        let response = KnowledgeFilesResponse::from_knowledge_and_files(updated, files);
        return Ok(HttpResponse::Ok().json(response));
    }

    Err(AppError::BadRequest(
        "File not in knowledge base".to_string(),
    ))
}

// POST /{id}/reset - Reset knowledge (delete all files and vector data)
async fn reset_knowledge(
    state: web::Data<AppState>,
    auth_user: AuthUser,
    knowledge_id: web::Path<String>,
) -> AppResult<HttpResponse> {
    let knowledge_service = KnowledgeService::new(&state.db);

    let knowledge = knowledge_service
        .get_knowledge_by_id(&knowledge_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Knowledge not found".to_string()))?;

    // Check write access
    if knowledge.user_id != auth_user.user.id && auth_user.user.role != "admin" {
        let group_service = GroupService::new(&state.db);
        let groups = group_service
            .get_groups_by_member_id(&auth_user.user.id)
            .await?;
        let user_group_ids: HashSet<String> = groups.into_iter().map(|g| g.id).collect();

        if !has_access(
            &auth_user.user.id,
            "write",
            &knowledge.access_control,
            &user_group_ids,
        ) {
            return Err(AppError::Forbidden("Access prohibited".to_string()));
        }
    }

    // TODO: Delete vector collection (requires vector DB integration)
    // Python equivalent:
    //   try:
    //       VECTOR_DB_CLIENT.delete_collection(collection_name=id)
    //   except Exception as e:
    //       log.debug(e)
    //       pass
    // This deletes all vector embeddings for the knowledge base

    // Reset file_ids to empty array
    let data = json!({"file_ids": []});
    let updated = knowledge_service
        .update_knowledge_data(&knowledge_id, data)
        .await?;

    Ok(HttpResponse::Ok().json(updated))
}

// POST /reindex - Reindex all knowledge files (admin only)
async fn reindex_all_knowledge(
    state: web::Data<AppState>,
    auth_user: AuthUser,
) -> AppResult<HttpResponse> {
    if auth_user.user.role != "admin" {
        return Err(AppError::Unauthorized("Unauthorized".to_string()));
    }

    let knowledge_service = KnowledgeService::new(&state.db);
    let file_service = FileService::new(&state.db);

    let knowledge_bases = knowledge_service.get_all_knowledge().await?;

    log::info!(
        "Starting reindexing for {} knowledge bases",
        knowledge_bases.len()
    );

    let mut deleted_knowledge_bases = Vec::new();

    for knowledge_base in knowledge_bases {
        // Robust error handling for missing or invalid data
        if knowledge_base.data.is_none() {
            log::warn!(
                "Knowledge base {} has no data. Deleting.",
                knowledge_base.id
            );
            if let Err(e) = knowledge_service.delete_knowledge(&knowledge_base.id).await {
                log::error!(
                    "Failed to delete invalid knowledge base {}: {}",
                    knowledge_base.id,
                    e
                );
            } else {
                deleted_knowledge_bases.push(knowledge_base.id.clone());
            }
            continue;
        }

        let data = knowledge_base.data.as_ref().unwrap();
        if !data.is_object() {
            log::warn!(
                "Knowledge base {} has invalid data: {:?}. Deleting.",
                knowledge_base.id,
                data
            );
            if let Err(e) = knowledge_service.delete_knowledge(&knowledge_base.id).await {
                log::error!(
                    "Failed to delete invalid knowledge base {}: {}",
                    knowledge_base.id,
                    e
                );
            } else {
                deleted_knowledge_bases.push(knowledge_base.id.clone());
            }
            continue;
        }

        // Get file IDs from knowledge base
        let file_ids = data
            .get("file_ids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        // Get files by IDs
        if let Ok(files) = file_service.get_files_by_ids(&file_ids).await {
            // TODO: Delete existing vector collection and reindex
            // try:
            //     if VECTOR_DB_CLIENT.has_collection(collection_name=knowledge_base.id):
            //         VECTOR_DB_CLIENT.delete_collection(collection_name=knowledge_base.id)
            // except Exception as e:
            //     log.error(f"Error deleting collection {knowledge_base.id}: {str(e)}")
            //     continue

            // Process each file
            for file in files {
                // TODO: Process file with vector DB
                // try:
                //     process_file(
                //         request,
                //         ProcessFileForm(file_id=file.id, collection_name=knowledge_base.id),
                //         user=user,
                //     )
                // except Exception as e:
                //     log.error(f"Error processing file {file.filename} (ID: {file.id}): {str(e)}")
                //     continue
                log::debug!(
                    "Would process file {} for knowledge base {}",
                    file.id,
                    knowledge_base.id
                );
            }
        }
    }

    log::info!(
        "Reindexing completed. Deleted {} invalid knowledge bases: {:?}",
        deleted_knowledge_bases.len(),
        deleted_knowledge_bases
    );

    Ok(HttpResponse::Ok().json(true))
}

// POST /{id}/files/batch/add - Add multiple files to knowledge
async fn add_files_batch(
    state: web::Data<AppState>,
    auth_user: AuthUser,
    knowledge_id: web::Path<String>,
    form: web::Json<Vec<KnowledgeFileIdForm>>,
) -> AppResult<HttpResponse> {
    let knowledge_service = KnowledgeService::new(&state.db);
    let file_service = FileService::new(&state.db);

    let knowledge = knowledge_service
        .get_knowledge_by_id(&knowledge_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Knowledge not found".to_string()))?;

    // Check write access
    if knowledge.user_id != auth_user.user.id && auth_user.user.role != "admin" {
        let group_service = GroupService::new(&state.db);
        let groups = group_service
            .get_groups_by_member_id(&auth_user.user.id)
            .await?;
        let user_group_ids: HashSet<String> = groups.into_iter().map(|g| g.id).collect();

        if !has_access(
            &auth_user.user.id,
            "write",
            &knowledge.access_control,
            &user_group_ids,
        ) {
            return Err(AppError::Forbidden("Access prohibited".to_string()));
        }
    }

    // TODO: Process files in batch and add to vector DB (requires vector DB integration)
    // Python equivalent:
    //   files: List[FileModel] = []
    //   for form in form_data:
    //       file = Files.get_file_by_id(form.file_id)
    //       if not file:
    //           raise HTTPException(status_code=400, detail=f"File {form.file_id} not found")
    //       files.append(file)
    //
    //   result = process_files_batch(
    //       request=request,
    //       form_data=BatchProcessFilesForm(files=files, collection_name=id),
    //       user=user
    //   )
    //
    //   successful_file_ids = [r.file_id for r in result.results if r.status == "completed"]
    //
    // This should:
    // 1. Validate all files exist and can be accessed
    // 2. Process all files in parallel/batch
    // 3. Generate embeddings for each file
    // 4. Store all embeddings in the knowledge base collection
    // 5. Return a list of successfully processed file IDs and any errors

    // Add file IDs to knowledge data
    let mut data = knowledge.data.clone().unwrap_or_else(|| json!({}));
    let mut file_ids = data
        .get("file_ids")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    for file_form in form.iter() {
        if !file_ids.contains(&file_form.file_id) {
            file_ids.push(file_form.file_id.clone());
        }
    }

    data["file_ids"] = json!(file_ids);
    let updated = knowledge_service
        .update_knowledge_data(&knowledge_id, data)
        .await?;

    // Get files
    let mut files = Vec::new();
    if let Some(data) = &updated.data {
        if let Some(file_ids) = data.get("file_ids").and_then(|v| v.as_array()) {
            let file_id_strings: Vec<String> = file_ids
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();

            for file_id in file_id_strings {
                if let Ok(Some(file)) = file_service.get_file_by_id(&file_id).await {
                    files.push(json!({
                        "id": file.id,
                        "filename": file.filename,
                        "meta": file.meta,
                        "created_at": file.created_at,
                        "updated_at": file.updated_at,
                    }));
                }
            }
        }
    }

    let response = KnowledgeFilesResponse::from_knowledge_and_files(updated, files);
    Ok(HttpResponse::Ok().json(response))
}
