//! Business feature/target semantics and provenance over Gail. Foresight owns
//! no model registry, trainer, qualification controller or resource executor.
use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use nm_workflow_core::{
    ApiError, App, Evaluation, Result, SourceRef, Value, digest, identifier, json, now, require,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureSchema {
    pub id: String,
    pub version: u32,
    pub names: Vec<String>,
    pub units: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feature {
    pub value: f64,
    pub observed_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Forecast {
    pub id: String,
    pub task: String,
    pub domain: String,
    pub source: SourceRef,
    pub dataset_digest: String,
    pub model_digest: String,
    pub evaluation_id: String,
    pub feature_schema: FeatureSchema,
    pub features: Vec<Feature>,
    pub observation_cutoff: i64,
    pub horizon_seconds: u64,
    pub output_unit: String,
    pub expires_at: i64,
}
pub fn validate_forecast(input: &Forecast, tenant: &str, clock: i64) -> Result<()> {
    input.source.validate(tenant, clock)?;
    require(
        identifier(&input.id)
            && identifier(&input.evaluation_id)
            && input.domain == "capacity"
            && matches!(
                (input.task.as_str(), input.output_unit.as_str()),
                ("compute_demand", "cpu_cores") | ("job_duration", "seconds")
            )
            && input.observation_cutoff > 0
            && input.observation_cutoff <= clock
            && clock - input.observation_cutoff <= 3600
            && input.horizon_seconds > 0
            && input.horizon_seconds <= 604800
            && input.expires_at > clock
            && input.expires_at <= clock + 3600
            && nm_workflow_core::evidence::valid_digest(&input.dataset_digest)
            && nm_workflow_core::evidence::valid_digest(&input.model_digest)
            && input.features.len() == input.feature_schema.names.len()
            && input.feature_schema.names.len() == input.feature_schema.units.len()
            && input.feature_schema.version > 0
            && identifier(&input.feature_schema.id)
            && !input.features.is_empty()
            && input.features.len() <= 256
            && input.feature_schema.names.iter().all(|v| identifier(v))
            && input
                .feature_schema
                .units
                .iter()
                .all(|v| !v.is_empty() && v.len() <= 64)
            && input.features.iter().all(|f| {
                f.value.is_finite()
                    && f.observed_at > 0
                    && f.observed_at <= input.observation_cutoff
                    && input.observation_cutoff - f.observed_at <= 3600
            }),
        "forecast semantics or time leakage",
    )
}
async fn forecast(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<Forecast>,
) -> Result<Json<Value>> {
    let actor = app.authorize(&headers, "use").await?;
    validate_forecast(&input, &actor.tenant, now())?;
    let source = app.source(&actor, &input.source, "forecast").await?;
    require(
        source.version == input.source.version,
        "forecast source version",
    )?;
    let fingerprint = digest(&input);
    if let Ok(previous) = app.store.get(&actor.tenant, "prediction", &input.id).await {
        if previous["actor"] != actor.principal || previous["request_digest"] != fingerprint {
            return Err(ApiError::conflict());
        }
        return Ok(Json(previous));
    }
    let feature_digest = digest(&input.feature_schema);
    let verity = App::upstream("verity", "http://verity.verity.svc.cluster.local:8080");
    let report = app
        .get(
            &format!("{verity}/v1/evaluations/{}", input.evaluation_id),
            &actor.bearer,
        )
        .await
        .ok()
        .and_then(|v| serde_json::from_value::<Evaluation>(v).ok());
    let qualified = report.as_ref().is_some_and(|e| {
        e.qualifies(
            &input.model_digest,
            &input.dataset_digest,
            &feature_digest,
            &input.task,
            &actor.tenant,
            &input.domain,
            now(),
        )
    });
    let mut result = json!({"schema_version":1,"id":input.id,"task":input.task,"actor":actor.principal,"tenant":actor.tenant,
        "request_digest":fingerprint,"dataset_digest":input.dataset_digest,"feature_digest":feature_digest,
        "model_digest":input.model_digest,"evaluation_id":input.evaluation_id,"source":input.source,
        "observation_cutoff":input.observation_cutoff,"horizon_seconds":input.horizon_seconds,
        "unit":input.output_unit,"expires_at":input.expires_at,
        "state":"abstained","reason":"qualification_unavailable","value":null,"uncertainty":null,"execution_authority":false});
    if qualified {
        let gail = App::upstream("gail", "http://gail.gail.svc.cluster.local:8080");
        let response=app.post(&format!("{gail}/v1/numerical/predict"),&App::upstream_token("gail"),
            &json!({"decision_id":input.id,"task":input.task,"tenant_scope":actor.tenant,
                "feature_schema":input.feature_schema,"feature_values":input.features.iter().map(|f|f.value).collect::<Vec<_>>(),
                "feature_timestamp":input.features.iter().map(|f|f.observed_at).min(),
                "target_unit":input.output_unit,"horizon_seconds":input.horizon_seconds,
                "expected_model_digest":input.model_digest})).await;
        match response {
            Ok(advice)
                if advice["applied"] == true
                    && advice["model_digest"] == input.model_digest
                    && advice["proposal"].as_f64().is_some_and(f64::is_finite)
                    && advice["uncertainty"]
                        .as_f64()
                        .is_some_and(|v| v.is_finite() && v >= 0.0) =>
            {
                result["state"] = json!("predicted");
                result["reason"] = Value::Null;
                result["value"] = advice["proposal"].clone();
                result["uncertainty"] = advice["uncertainty"].clone();
                result["gail"] = advice;
            }
            Ok(advice) => {
                result["reason"] = json!("gail_abstained");
                result["gail"] = advice;
            }
            Err(_) => {
                result["reason"] = json!("gail_unavailable");
            }
        }
    }
    app.store
        .insert(
            &actor.tenant,
            "prediction",
            &input.id,
            &actor.principal,
            &result,
        )
        .await?;
    Ok(Json(result))
}
async fn prediction(
    State(app): State<App>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let actor = app.authorize(&headers, "observe").await?;
    let value = app.store.get(&actor.tenant, "prediction", &id).await?;
    let source: SourceRef =
        serde_json::from_value(value["source"].clone()).map_err(|_| ApiError::unavailable())?;
    if value["actor"] != actor.principal || !source.permits(&actor, "forecast", now()) {
        return Err(ApiError::forbidden());
    }
    app.source(&actor, &source, "forecast").await?;
    Ok(Json(value))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    prediction_id: String,
    capacity_cpu_cores: f64,
    reserved_cpu_cores: f64,
}
async fn scenario(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<Scenario>,
) -> Result<Json<Value>> {
    let actor = app.authorize(&headers, "use").await?;
    require(
        input.capacity_cpu_cores.is_finite()
            && input.reserved_cpu_cores.is_finite()
            && input.capacity_cpu_cores >= 0.0
            && input.reserved_cpu_cores >= 0.0,
        "scenario bounds",
    )?;
    let record = app
        .store
        .get(&actor.tenant, "prediction", &input.prediction_id)
        .await?;
    if record["actor"] != actor.principal {
        return Err(ApiError::forbidden());
    }
    let source: SourceRef =
        serde_json::from_value(record["source"].clone()).map_err(|_| ApiError::unavailable())?;
    app.source(&actor, &source, "forecast").await?;
    require(
        source.permits(&actor, "forecast", now()) && record["unit"] == "cpu_cores",
        "scenario source and units",
    )?;
    if record["state"] != "predicted" || record["expires_at"].as_i64().unwrap_or(0) <= now() {
        return Ok(Json(
            json!({"state":"unavailable","reason":"qualified_prediction_unavailable"}),
        ));
    }
    let predicted = record["value"].as_f64().ok_or_else(ApiError::unavailable)?;
    let radius = record["uncertainty"]
        .as_f64()
        .ok_or_else(ApiError::unavailable)?;
    Ok(Json(
        json!({"state":"compared","prediction_id":input.prediction_id,
        "assumptions":{"capacity_cpu_cores":input.capacity_cpu_cores,"reserved_cpu_cores":input.reserved_cpu_cores},
        "estimated_headroom_cpu_cores":input.capacity_cpu_cores-input.reserved_cpu_cores-predicted,
        "conservative_headroom_cpu_cores":input.capacity_cpu_cores-input.reserved_cpu_cores-predicted-radius,
        "causal_claim":false,"execution_authority":false}),
    ))
}
async fn capabilities(State(app): State<App>, headers: HeaderMap) -> Result<Json<Value>> {
    app.authorize(&headers, "observe").await?;
    let gail = App::upstream("gail", "http://gail.gail.svc.cluster.local:8080");
    let status = app
        .get(
            &format!("{gail}/v1/elm/status"),
            &App::upstream_token("gail"),
        )
        .await
        .unwrap_or_else(|_| json!({"state":"unavailable"}));
    Ok(Json(
        json!({"tasks":["compute_demand","job_duration"],"domain":"capacity",
        "inference":"gail","qualification":"gail_and_exact_verity_evidence","gail":status,
        "classification":"unavailable","ranking":"unavailable","constrained_optimisation":"unavailable"}),
    ))
}
pub fn routes() -> Router<App> {
    Router::new()
        .route("/v1/predictions", post(forecast))
        .route("/v1/predictions/{id}", get(prediction))
        .route("/v1/scenarios", post(scenario))
        .route("/v1/capabilities", get(capabilities))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> Forecast {
        Forecast {
            id: "p1".into(),
            task: "compute_demand".into(),
            domain: "capacity".into(),
            source: SourceRef {
                source_id: "tracey".into(),
                object_id: "cpu".into(),
                version: "v1".into(),
                tenant: "t".into(),
                readers: vec!["alice".into()],
                purposes: vec!["forecast".into()],
                observed_at: 1,
                access_expires_at: 5000,
                deleted: false,
                training_permitted: false,
            },
            dataset_digest: "a".repeat(64),
            model_digest: "b".repeat(64),
            evaluation_id: "e1".into(),
            feature_schema: FeatureSchema {
                id: "capacity-v1".into(),
                version: 1,
                names: vec!["cpu".into()],
                units: vec!["cpu_cores".into()],
            },
            features: vec![Feature {
                value: 1.0,
                observed_at: 50,
            }],
            observation_cutoff: 100,
            horizon_seconds: 3600,
            output_unit: "cpu_cores".into(),
            expires_at: 200,
        }
    }
    #[test]
    fn future_data_stale_input_and_wrong_units_fail_closed() {
        let mut r = request();
        assert!(validate_forecast(&r, "t", 100).is_ok());
        r.features[0].observed_at = 101;
        assert!(validate_forecast(&r, "t", 100).is_err());
        r = request();
        r.output_unit = "GBP".into();
        assert!(validate_forecast(&r, "t", 100).is_err());
        r = request();
        r.observation_cutoff = 1;
        r.expires_at = 5000;
        assert!(validate_forecast(&r, "t", 4000).is_err());
    }
}
