use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::dto::matches::{
    delete_question_command, AdjustScoreRequest, CreateMatchRequest, CsvInvalidRowResponse,
    ImportMatchQuestionsRequest, ImportQuestionsResponse, JoinMatchRequest, JoinMatchResponse,
    LiveRoomResponse, MatchParticipantResponse, MatchQuestionRequest, MatchQuestionResponse,
    MatchResponse, MatchRoundAnswerResponse, MatchRoundDetailResponse, MatchRoundResponse,
    MatchSettingsResponse, MatchStateResponse, MatchSummaryResponse, MatchTopScoreResponse,
    PlayerAnswerRequest, PlayerAnswerResponse, RegisterAnswerRequest,
};
use crate::api::dto::rooms::JoinRoomRequest;
use crate::api::extractors::AuthenticatedUser;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::matches::{
    AdjustScoreCommand, BuzzCommand, CreateMatchCommand, DrawRoundCommand, EndMatchCommand,
    JoinByPinCommand, LiveRoomState, MatchQuestionState, MatchRepository, MatchRoundDetailState,
    MatchRoundState, MatchSettingsState, MatchState, MatchSummaryState, MatchTopScoreState,
    MatchUseCases, RegisterAnswerCommand, ResetMatchCommand, RevealAnswerCommand,
    StartMatchCommand,
};
use crate::domain::matches::{JoinSource, Match, MatchParticipant, MatchStatus};
use crate::infrastructure::repositories::mysql_matches::MySqlMatchRepository;
use crate::infrastructure::repositories::mysql_notifications::MySqlNotificationRepository;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/matches", post(create_match).get(list_matches))
        .route("/matches/:match_id", get(get_match))
        .route("/matches/:match_id/state", get(get_match_state))
        .route("/matches/:match_id/rounds", get(list_round_details))
        .route(
            "/matches/:match_id/participants",
            get(list_participants).post(add_participant),
        )
        .route(
            "/matches/:match_id/questions",
            get(list_questions).post(add_question),
        )
        .route(
            "/matches/:match_id/questions/import-csv",
            post(import_questions),
        )
        .route(
            "/matches/:match_id/questions/:question_id",
            patch(update_question).delete(delete_question),
        )
        .route("/matches/:match_id/join", post(join_match))
        .route("/matches/:match_id/start", post(start_match))
        .route("/matches/:match_id/draw", post(draw_round))
        .route("/matches/:match_id/end", post(end_match))
        .route("/matches/:match_id/reset", post(reset_match))
        .route("/matches/:match_id/rounds/:round_id/buzz", post(buzz))
        .route(
            "/matches/:match_id/rounds/:round_id/reveal",
            post(reveal_answer),
        )
        .route(
            "/matches/:match_id/rounds/:round_id/answer",
            post(register_answer),
        )
        .route(
            "/matches/:match_id/rounds/:round_id/player-answer",
            post(player_answer),
        )
        .route(
            "/matches/:match_id/participants/:participant_id/score",
            post(adjust_score),
        )
        .route("/rooms/join", post(join_room))
}

async fn list_matches(State(state): State<AppState>, auth: AuthenticatedUser) -> Response {
    let repository = match match_repository(state) {
        Ok(repository) => repository,
        Err(error) => return app_error(error),
    };
    match repository.list_summaries_by_owner(&auth.user_id).await {
        Ok(matches) => Json(
            matches
                .into_iter()
                .map(match_summary_response)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn get_match(State(state): State<AppState>, Path(match_id): Path<String>) -> Response {
    let repository = match match_repository(state) {
        Ok(repository) => repository,
        Err(error) => return app_error(error),
    };
    match repository.find_by_id(&match_id).await {
        Ok(Some(match_entity)) => Json(match_response(match_entity)).into_response(),
        Ok(None) => app_error(crate::errors::AppError::NotFound(
            "match was not found".to_owned(),
        )),
        Err(error) => app_error(error),
    }
}

async fn get_match_state(State(state): State<AppState>, Path(match_id): Path<String>) -> Response {
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases.get_state(&match_id).await {
        Ok(Some(state)) => Json(match_state_response(state)).into_response(),
        Ok(None) => app_error(crate::errors::AppError::NotFound(
            "match was not found".to_owned(),
        )),
        Err(error) => app_error(error),
    }
}

async fn list_round_details(
    State(state): State<AppState>,
    Path(match_id): Path<String>,
) -> Response {
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases.list_round_details(&match_id).await {
        Ok(rounds) => Json(
            rounds
                .into_iter()
                .map(match_round_detail_response)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn create_match(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(request): Json<CreateMatchRequest>,
) -> Response {
    let mut command: CreateMatchCommand = request.into();
    command.host_user_id = auth.user_id;
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases.create_match(command).await {
        Ok(created_match) => {
            (StatusCode::CREATED, Json(match_response(created_match))).into_response()
        }
        Err(error) => app_error(error),
    }
}

async fn list_participants(
    State(state): State<AppState>,
    Path(match_id): Path<String>,
) -> Response {
    let repository = match match_repository(state) {
        Ok(repository) => repository,
        Err(error) => return app_error(error),
    };
    match repository.list_participants(&match_id).await {
        Ok(participants) => Json(
            participants
                .into_iter()
                .map(participant_response)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn add_participant(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(match_id): Path<String>,
    Json(request): Json<crate::api::dto::matches::AddParticipantRequest>,
) -> Response {
    let repository = match match_repository(state.clone()) {
        Ok(repository) => repository,
        Err(error) => return app_error(error),
    };
    if let Err(error) = require_match_host(&repository, &auth, &match_id).await {
        return app_error(error);
    }
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases
        .add_participant(request.into_command(match_id))
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn list_questions(State(state): State<AppState>, Path(match_id): Path<String>) -> Response {
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases.get_state(&match_id).await {
        Ok(Some(state)) => Json(
            state
                .questions
                .into_iter()
                .map(question_response)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Ok(None) => app_error(crate::errors::AppError::NotFound(
            "match was not found".to_owned(),
        )),
        Err(error) => app_error(error),
    }
}

async fn add_question(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(match_id): Path<String>,
    Json(request): Json<MatchQuestionRequest>,
) -> Response {
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases
        .add_match_question(request.into_add_command(match_id, auth.user_id))
        .await
    {
        Ok(()) => StatusCode::CREATED.into_response(),
        Err(error) => app_error(error),
    }
}

async fn update_question(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((match_id, question_id)): Path<(String, String)>,
    Json(request): Json<MatchQuestionRequest>,
) -> Response {
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases
        .update_match_question(request.into_update_command(match_id, question_id, auth.user_id))
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn delete_question(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((match_id, question_id)): Path<(String, String)>,
) -> Response {
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases
        .delete_match_question(delete_question_command(match_id, question_id, auth.user_id))
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn import_questions(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(match_id): Path<String>,
    Json(request): Json<ImportMatchQuestionsRequest>,
) -> Response {
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases
        .import_match_questions(request.into_command(match_id, auth.user_id))
        .await
    {
        Ok(report) => Json(ImportQuestionsResponse {
            created: report.created,
            invalid_rows: report
                .invalid_rows
                .into_iter()
                .map(|row| CsvInvalidRowResponse {
                    line: row.line,
                    reason: row.reason,
                })
                .collect(),
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn start_match(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(match_id): Path<String>,
) -> Response {
    let use_cases = match match_use_cases(state.clone()) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases
        .start_match(StartMatchCommand {
            match_id: match_id.clone(),
            host_user_id: auth.user_id.clone(),
        })
        .await
    {
        Ok(()) => {
            emit_match_started_notification(&state, &auth.user_id, &match_id).await;
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => app_error(error),
    }
}

async fn emit_match_started_notification(state: &AppState, host_user_id: &str, match_id: &str) {
    let Ok(db) = state.db() else {
        return;
    };
    let match_repository = MySqlMatchRepository::new(db.clone());
    let Ok(Some(match_entity)) = match_repository.find_by_id(match_id).await else {
        return;
    };
    let name = match_entity.name.as_str().to_owned();

    let notifications = MySqlNotificationRepository::new(db);
    let enabled = notifications
        .is_enabled(host_user_id, "new_rooms")
        .await
        .unwrap_or(true);
    if !enabled {
        return;
    }
    let _ = notifications
        .create(
            Some(host_user_id),
            "new_rooms",
            &format!("Partida iniciada: {name}"),
            Some("Uma nova partida está ao vivo."),
            Some(&format!("/matches/{match_id}/host")),
        )
        .await;
}

async fn draw_round(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(match_id): Path<String>,
) -> Response {
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases
        .draw_question(DrawRoundCommand {
            match_id,
            host_user_id: auth.user_id,
        })
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn join_room(
    State(state): State<AppState>,
    Json(request): Json<JoinRoomRequest>,
) -> Response {
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    let display_name = request
        .participant_name
        .unwrap_or_else(|| "Jogador remoto".to_owned());
    match use_cases
        .join_by_pin(JoinByPinCommand {
            pin: request.pin,
            player_id: None,
            display_name,
        })
        .await
    {
        Ok(joined) => Json(JoinMatchResponse {
            participant: participant_response(joined.participant),
            participant_token: joined.participant_token,
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn join_match(
    State(state): State<AppState>,
    Path(match_id): Path<String>,
    Json(request): Json<JoinMatchRequest>,
) -> Response {
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases
        .join_match_by_id(request.into_command(match_id))
        .await
    {
        Ok(joined) => Json(JoinMatchResponse {
            participant: participant_response(joined.participant),
            participant_token: joined.participant_token,
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn buzz(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path((match_id, round_id)): Path<(String, String)>,
    Json(request): Json<BuzzRequest>,
) -> Response {
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases
        .buzz(BuzzCommand {
            match_id,
            round_id,
            participant_id: request.participant_id,
            presence_session_id: request.presence_session_id,
        })
        .await
    {
        Ok(accepted) => Json(BuzzResponse { accepted }).into_response(),
        Err(error) => app_error(error),
    }
}

async fn reveal_answer(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((match_id, round_id)): Path<(String, String)>,
) -> Response {
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases
        .reveal_answer(RevealAnswerCommand {
            match_id,
            round_id,
            host_user_id: auth.user_id,
        })
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn register_answer(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((match_id, round_id)): Path<(String, String)>,
    Json(request): Json<RegisterAnswerRequest>,
) -> Response {
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    let command = RegisterAnswerCommand {
        match_id,
        round_id,
        participant_id: request.participant_id,
        result: request.result,
        actor_user_id: auth.user_id,
    };
    match use_cases.register_answer(command).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn player_answer(
    State(state): State<AppState>,
    Path((match_id, round_id)): Path<(String, String)>,
    Json(request): Json<PlayerAnswerRequest>,
) -> Response {
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases
        .player_answer(request.into_command(match_id, round_id))
        .await
    {
        Ok(outcome) => Json(PlayerAnswerResponse {
            accepted: outcome.accepted,
            correct: outcome.correct,
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn adjust_score(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((match_id, participant_id)): Path<(String, String)>,
    Json(request): Json<AdjustScoreRequest>,
) -> Response {
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases
        .adjust_score(AdjustScoreCommand {
            match_id,
            participant_id,
            actor_user_id: auth.user_id,
            delta: request.delta,
            reason: request.reason,
        })
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn end_match(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(match_id): Path<String>,
) -> Response {
    let use_cases = match match_use_cases(state.clone()) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases
        .end_match(EndMatchCommand {
            match_id: match_id.clone(),
            host_user_id: auth.user_id.clone(),
        })
        .await
    {
        Ok(()) => {
            emit_match_ended_notification(&state, &auth.user_id, &match_id).await;
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => app_error(error),
    }
}

async fn emit_match_ended_notification(state: &AppState, host_user_id: &str, match_id: &str) {
    let Ok(db) = state.db() else {
        return;
    };
    let match_repository = MySqlMatchRepository::new(db.clone());
    let Ok(Some(snapshot)) = match_repository.get_state(match_id).await else {
        return;
    };
    let name = snapshot.match_entity.name.as_str().to_owned();
    let body = match snapshot.winner {
        Some(winner) => format!(
            "Vencedor: {} · {} pts",
            winner.display_name_snapshot.as_str(),
            winner.current_score.value()
        ),
        None => "Partida encerrada sem vencedor.".to_owned(),
    };

    let notifications = MySqlNotificationRepository::new(db);
    let enabled = notifications
        .is_enabled(host_user_id, "ranking_updates")
        .await
        .unwrap_or(true);
    if !enabled {
        return;
    }
    let _ = notifications
        .create(
            Some(host_user_id),
            "ranking_updates",
            &format!("Partida encerrada: {name}"),
            Some(&body),
            Some("/history"),
        )
        .await;
}

async fn reset_match(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(match_id): Path<String>,
) -> Response {
    let use_cases = match match_use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(error) => return app_error(error),
    };
    match use_cases
        .reset_match(ResetMatchCommand {
            match_id,
            host_user_id: auth.user_id,
        })
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

fn match_use_cases(
    state: AppState,
) -> Result<
    MatchUseCases<MySqlMatchRepository, crate::realtime::RealtimeHub>,
    crate::errors::AppError,
> {
    Ok(MatchUseCases::new_with_realtime(
        MySqlMatchRepository::new(state.db()?),
        state.realtime,
    ))
}

fn match_repository(state: AppState) -> Result<MySqlMatchRepository, crate::errors::AppError> {
    Ok(MySqlMatchRepository::new(state.db()?))
}

async fn require_match_host(
    repository: &MySqlMatchRepository,
    auth: &AuthenticatedUser,
    match_id: &str,
) -> Result<(), crate::errors::AppError> {
    let Some(match_entity) = repository.find_by_id(match_id).await? else {
        return Err(crate::errors::AppError::NotFound(
            "match was not found".to_owned(),
        ));
    };
    if match_entity.host_user_id.as_str() != auth.user_id {
        return Err(crate::errors::AppError::Forbidden(
            "host does not own this match".to_owned(),
        ));
    }
    Ok(())
}

fn match_response(value: Match) -> MatchResponse {
    MatchResponse {
        id: value.id.as_str().to_owned(),
        game_id: value.game_id.as_str().to_owned(),
        game_mode_id: value.game_mode_id.as_str().to_owned(),
        host_user_id: value.host_user_id.as_str().to_owned(),
        name: value.name.as_str().to_owned(),
        status: match_status(value.status).to_owned(),
        current_round_id: value.current_round_id.map(|id| id.as_str().to_owned()),
        winner_participant_id: value.winner_participant_id.map(|id| id.as_str().to_owned()),
    }
}

fn match_summary_response(value: MatchSummaryState) -> MatchSummaryResponse {
    let match_entity = value.match_entity;
    MatchSummaryResponse {
        id: match_entity.id.as_str().to_owned(),
        game_id: match_entity.game_id.as_str().to_owned(),
        game_mode_id: match_entity.game_mode_id.as_str().to_owned(),
        host_user_id: match_entity.host_user_id.as_str().to_owned(),
        name: match_entity.name.as_str().to_owned(),
        status: match_status(match_entity.status).to_owned(),
        current_round_id: match_entity
            .current_round_id
            .map(|id| id.as_str().to_owned()),
        winner_participant_id: match_entity
            .winner_participant_id
            .map(|id| id.as_str().to_owned()),
        game_name: value.game_name,
        mode_name: value.mode_name,
        question_count: value.question_count,
        participant_count: value.participant_count,
        rounds_played: value.rounds_played,
        current_round_number: value.current_round_number,
        current_round_status: value.current_round_status,
        started_at: match_entity
            .started_at
            .map(|datetime| datetime.as_str().to_owned()),
        ended_at: match_entity
            .ended_at
            .map(|datetime| datetime.as_str().to_owned()),
        duration_seconds: value.duration_seconds,
        winner_name: value.winner_name,
        winner_score: value.winner_score,
        top_scores: value
            .top_scores
            .into_iter()
            .map(match_top_score_response)
            .collect(),
    }
}

fn match_top_score_response(value: MatchTopScoreState) -> MatchTopScoreResponse {
    MatchTopScoreResponse {
        participant_id: value.participant_id,
        display_name: value.display_name,
        score: value.score,
    }
}

fn match_round_detail_response(value: MatchRoundDetailState) -> MatchRoundDetailResponse {
    MatchRoundDetailResponse {
        round_number: value.round_number,
        status: value.status,
        prompt: value.prompt,
        expected_answer: value.expected_answer,
        answers: value
            .answers
            .into_iter()
            .map(|answer| MatchRoundAnswerResponse {
                participant_id: answer.participant_id,
                display_name: answer.display_name,
                result: answer.result,
                points_applied: answer.points_applied,
            })
            .collect(),
    }
}

fn participant_response(value: MatchParticipant) -> MatchParticipantResponse {
    MatchParticipantResponse {
        id: value.id.as_str().to_owned(),
        match_id: value.match_id.as_str().to_owned(),
        player_id: value.player_id.map(|id| id.as_str().to_owned()),
        display_name: value.display_name_snapshot.as_str().to_owned(),
        join_source: join_source(value.join_source).to_owned(),
        current_score: value.current_score.value(),
        lives_remaining: value.lives_remaining,
        eliminated: value.eliminated,
    }
}

fn match_state_response(value: MatchState) -> MatchStateResponse {
    MatchStateResponse {
        match_data: match_response(value.match_entity),
        settings: settings_response(value.settings),
        questions: value.questions.into_iter().map(question_response).collect(),
        participants: value
            .participants
            .into_iter()
            .map(participant_response)
            .collect(),
        current_round: value.current_round.map(round_response),
        room: value.room.map(live_room_response),
        winner: value.winner.map(participant_response),
    }
}

fn settings_response(value: MatchSettingsState) -> MatchSettingsResponse {
    MatchSettingsResponse {
        mode: value.mode,
        game_kind: value.game_kind,
        max_rounds: value.max_rounds,
        time_per_round_seconds: value.time_per_round_seconds,
        initial_points: value.initial_points,
        remote_enabled: value.remote_enabled,
        capacity: value.capacity,
    }
}

fn question_response(value: MatchQuestionState) -> MatchQuestionResponse {
    MatchQuestionResponse {
        id: value.id,
        prompt: value.prompt,
        expected_answer: value.expected_answer,
        points: value.points,
        used: value.used,
        position: value.position,
    }
}

fn round_response(value: MatchRoundState) -> MatchRoundResponse {
    MatchRoundResponse {
        id: value.id,
        match_question_id: value.match_question_id,
        round_number: value.round_number,
        status: value.status,
        selected_participant_id: value.selected_participant_id,
        answer_revealed: value.answer_revealed,
        time_remaining_seconds: value.time_remaining_seconds,
        question: value.question.map(question_response),
    }
}

fn live_room_response(value: LiveRoomState) -> LiveRoomResponse {
    LiveRoomResponse {
        id: value.id,
        match_id: value.match_id,
        match_name: value.match_name,
        game_id: value.game_id,
        game_name: value.game_name,
        pin: value.pin,
        status: value.status,
        participants: value.participants,
        capacity: value.capacity,
        host_name: value.host_name,
        started_minutes_ago: value.started_minutes_ago,
    }
}

fn match_status(value: MatchStatus) -> &'static str {
    match value {
        MatchStatus::Waiting => "waiting",
        MatchStatus::InProgress => "in_progress",
        MatchStatus::Finished => "finished",
        MatchStatus::Cancelled => "cancelled",
    }
}

fn join_source(value: JoinSource) -> &'static str {
    match value {
        JoinSource::Local => "local",
        JoinSource::Remote => "remote",
    }
}

#[derive(Debug, Deserialize)]
struct BuzzRequest {
    participant_id: String,
    presence_session_id: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct BuzzResponse {
    accepted: bool,
}
