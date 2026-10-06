//! Offline checks for the SDK's public construction and JSON contracts.
use serde_json::{Value, json};
use typesafe_jev_sdk::{Answer, JevRequest, JevResponse, Question, ResponseValidationError, Usage};

#[test]
fn optional_response_validation_rejects_semantic_errors() {
    let mut request = JevRequest::new("jev-latest", "state");
    request.add_question("choice", Question::choice("pick", vec![("a", "A")]));
    let answer = json!({"type":"choice","choice":"a","confidence":0.8,"probabilities":{"a":1.0}});
    let make_response = |answer: Value| -> JevResponse {
        serde_json::from_value(json!({
            "model":"jev-latest", "usage":{"input_tokens":1,"output_tokens":1},
            "answers":{"choice":answer}
        }))
        .unwrap()
    };
    assert!(
        make_response(answer.clone())
            .validate_against(&request)
            .is_ok()
    );
    let mut bad = answer.clone();
    bad["choice"] = json!("other");
    assert!(matches!(
        make_response(bad).validate_against(&request),
        Err(ResponseValidationError::UnknownChoice(_))
    ));
    let mut bad = answer.clone();
    bad["confidence"] = json!(1.1);
    assert!(matches!(
        make_response(bad).validate_against(&request),
        Err(ResponseValidationError::InvalidValue(_))
    ));
    let mut bad = answer.clone();
    bad["probabilities"] = json!({"other":0.5});
    assert!(matches!(
        make_response(bad).validate_against(&request),
        Err(ResponseValidationError::UnknownChoice(_))
    ));
    let mut bad = answer;
    bad["type"] = json!("noul");
    bad["noul"] = json!(0.5);
    assert!(matches!(
        make_response(bad).validate_against(&request),
        Err(ResponseValidationError::MismatchedAnswerType(_))
    ));
    let missing: JevResponse = serde_json::from_value(json!({
        "model":"jev-latest", "usage":{"input_tokens":1,"output_tokens":1},"answers":{}
    }))
    .unwrap();
    assert!(matches!(
        missing.validate_against(&request),
        Err(ResponseValidationError::MissingAnswer(_))
    ));
}

fn assert_question_round_trip(question: Question, expected: Value) {
    let encoded = serde_json::to_value(&question).unwrap();
    assert_eq!(encoded, expected);
    assert_eq!(
        serde_json::from_value::<Question>(encoded).unwrap(),
        question
    );
}

#[test]
fn choice_constructor_preserves_unicode_and_last_duplicate_value() {
    assert_question_round_trip(
        Question::choice(
            "次の行動？\n\"安全\"を優先",
            vec![
                ("回復", "old"),
                ("退避", "距離を取る"),
                ("回復", "HPを回復"),
            ],
        ),
        json!({
            "type": "choice",
            "instructions": "次の行動？\n\"安全\"を優先",
            "criteria": {"回復": "HPを回復", "退避": "距離を取る"}
        }),
    );
}

#[test]
fn score_constructor_preserves_order_and_duplicates() {
    assert_question_round_trip(
        Question::score(String::from("risk"), vec!["high", "low", "high"]),
        json!({"type": "score", "instructions": "risk", "criteria": ["high", "low", "high"]}),
    );
}

#[test]
fn noul_constructor_omits_absent_criteria() {
    assert_question_round_trip(
        Question::noul("heal?", None::<Vec<(&str, &str)>>),
        json!({"type": "noul", "instructions": "heal?"}),
    );
}

#[test]
fn noul_constructor_keeps_present_empty_criteria() {
    assert_question_round_trip(
        Question::noul("heal?", Some(Vec::<(&str, &str)>::new())),
        json!({"type": "noul", "instructions": "heal?", "criteria": {}}),
    );
}

#[test]
fn noul_constructor_accepts_owned_strings_and_replaces_duplicates() {
    assert_question_round_trip(
        Question::noul(
            String::from("heal?"),
            Some(vec![
                (String::from("true"), String::from("old")),
                (String::from("true"), String::from("yes")),
                (String::from("false"), String::from("no")),
            ]),
        ),
        json!({"type": "noul", "instructions": "heal?", "criteria": {"true": "yes", "false": "no"}}),
    );
}

#[test]
fn constructors_preserve_empty_inputs_without_imposing_validation() {
    assert_question_round_trip(
        Question::choice("", Vec::<(&str, &str)>::new()),
        json!({"type": "choice", "instructions": "", "criteria": {}}),
    );
    assert_question_round_trip(
        Question::score("", Vec::<String>::new()),
        json!({"type": "score", "instructions": "", "criteria": []}),
    );
}

#[test]
fn noul_null_criteria_normalizes_to_omitted_criteria() {
    let question: Question = serde_json::from_value(json!({
        "type": "noul", "instructions": "heal?", "criteria": null
    }))
    .unwrap();
    assert_eq!(question, Question::noul("heal?", None::<Vec<(&str, &str)>>));
    assert_eq!(
        serde_json::to_value(question).unwrap(),
        json!({"type": "noul", "instructions": "heal?"})
    );
}

#[test]
fn malformed_questions_are_rejected() {
    let cases = [
        json!({"instructions": "q", "criteria": {}}),
        json!({"type": "unknown", "instructions": "q"}),
        json!({"type": "Choice", "instructions": "q", "criteria": {}}),
        json!({"type": "choice", "criteria": {}}),
        json!({"type": "choice", "instructions": "q"}),
        json!({"type": "choice", "instructions": "q", "criteria": []}),
        json!({"type": "score", "instructions": "q", "criteria": {}}),
        json!({"type": "score", "instructions": "q", "criteria": [1]}),
        json!({"type": "noul", "instructions": null}),
        json!({"type": "noul", "instructions": "q", "criteria": {"true": true}}),
    ];
    for payload in cases {
        assert!(
            serde_json::from_value::<Question>(payload.clone()).is_err(),
            "accepted {payload}"
        );
    }
}

#[test]
fn request_constructor_and_replacement_have_exact_wire_shape() {
    let mut request = JevRequest::new(String::from("custom-model"), "状態\nline two");
    assert!(request.questions.is_empty());
    request.add_question("decision", Question::score("old", vec!["low"]));
    request.add_question(
        String::from("decision"),
        Question::choice("new", vec![("go", "進む")]),
    );
    request.add_question("check", Question::noul("ready?", None::<Vec<(&str, &str)>>));
    assert_eq!(request.questions.len(), 2);
    let expected = json!({
        "model": "custom-model", "state": "状態\nline two",
        "questions": {
            "decision": {"type": "choice", "instructions": "new", "criteria": {"go": "進む"}},
            "check": {"type": "noul", "instructions": "ready?"}
        }
    });
    assert_eq!(serde_json::to_value(&request).unwrap(), expected);
    let decoded: JevRequest = serde_json::from_value(expected.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), expected);
}

#[test]
fn request_constructor_allows_empty_strings_and_questions() {
    assert_eq!(
        serde_json::to_value(JevRequest::new("", "")).unwrap(),
        json!({
            "model": "", "state": "", "questions": {}
        })
    );
}

#[test]
fn request_requires_all_wire_fields() {
    for missing in ["model", "state", "questions"] {
        let mut payload = json!({"model": "jev-latest", "state": "s", "questions": {}});
        payload.as_object_mut().unwrap().remove(missing);
        assert!(
            serde_json::from_value::<JevRequest>(payload).is_err(),
            "accepted missing {missing}"
        );
    }
}

#[test]
fn all_answer_variants_round_trip_with_exact_wire_shape() {
    let cases = [
        json!({"type": "choice", "choice": "go", "confidence": 1.0, "probabilities": {"go": 1.0, "stop": 0.0}}),
        json!({"type": "score", "score": 2.5, "confidence": 0.0, "legend": {"2": "中"}, "probabilities": {"2": 0.5}}),
        json!({"type": "noul", "noul": 0.0}),
        json!({"type": "noul", "noul": 1.0}),
    ];
    for payload in cases {
        let answer: Answer = serde_json::from_value(payload.clone()).unwrap();
        assert_eq!(serde_json::to_value(&answer).unwrap(), payload);
        assert_eq!(
            serde_json::from_str::<Answer>(&serde_json::to_string(&answer).unwrap()).unwrap(),
            answer
        );
    }
}

#[test]
fn answers_require_the_fields_of_their_variant() {
    let cases = [
        (
            json!({"type": "choice", "choice": "go", "confidence": 1.0, "probabilities": {}}),
            vec!["type", "choice", "confidence", "probabilities"],
        ),
        (
            json!({"type": "score", "score": 1.0, "confidence": 1.0, "legend": {}, "probabilities": {}}),
            vec!["type", "score", "confidence", "legend", "probabilities"],
        ),
        (json!({"type": "noul", "noul": 1.0}), vec!["type", "noul"]),
    ];
    for (payload, fields) in cases {
        for field in fields {
            let mut incomplete = payload.clone();
            incomplete.as_object_mut().unwrap().remove(field);
            assert!(
                serde_json::from_value::<Answer>(incomplete).is_err(),
                "accepted missing {field} in {payload}"
            );
        }
    }
}

#[test]
fn malformed_answers_are_rejected() {
    for payload in [
        json!({"type": "unknown", "noul": 1.0}),
        json!({"type": "Noul", "noul": 1.0}),
        json!({"type": "noul", "noul": "1.0"}),
        json!({"type": "noul", "noul": null}),
        json!({"type": "choice", "choice": 1, "confidence": 1.0, "probabilities": {}}),
        json!({"type": "choice", "choice": "go", "confidence": 1.0, "probabilities": {"go": "1"}}),
        json!({"type": "score", "score": 1.0, "confidence": 1.0, "legend": [], "probabilities": {}}),
    ] {
        assert!(
            serde_json::from_value::<Answer>(payload.clone()).is_err(),
            "accepted {payload}"
        );
    }
}

#[test]
fn complete_response_round_trips_and_exposes_usage() {
    let payload = json!({
        "model": "jev-latest", "usage": {"input_tokens": 12, "output_tokens": 3},
        "answers": {
            "move": {"type": "choice", "choice": "go", "confidence": 0.5, "probabilities": {"go": 0.5}},
            "risk": {"type": "score", "score": 2.0, "confidence": 1.0, "legend": {}, "probabilities": {}},
            "ready": {"type": "noul", "noul": 1.0}
        }
    });
    let response: JevResponse = serde_json::from_value(payload.clone()).unwrap();
    assert_eq!(response.model, "jev-latest");
    assert_eq!(response.usage.tokens(), 15);
    assert_eq!(response.answers.len(), 3);
    assert_eq!(response.answers["ready"], Answer::Noul { noul: 1.0 });
    assert_eq!(serde_json::to_value(response).unwrap(), payload);
}

#[test]
fn response_accepts_empty_answers_and_unknown_fields() {
    let response: JevResponse = serde_json::from_value(json!({
        "model": "future", "usage": {"input_tokens": 0, "output_tokens": 0, "new_metric": 4},
        "answers": {}, "request_id": "future-field"
    }))
    .unwrap();
    assert!(response.answers.is_empty());
    assert_eq!(response.usage.tokens(), 0);
}

#[test]
fn response_rejects_missing_fields_and_invalid_nested_data() {
    for missing in ["model", "usage", "answers"] {
        let mut payload = json!({"model": "jev-latest", "usage": {"input_tokens": 0, "output_tokens": 0}, "answers": {}});
        payload.as_object_mut().unwrap().remove(missing);
        assert!(
            serde_json::from_value::<JevResponse>(payload).is_err(),
            "accepted missing {missing}"
        );
    }
    for payload in [
        json!({"model": "jev-latest", "usage": {"input_tokens": -1, "output_tokens": 0}, "answers": {}}),
        json!({"model": "jev-latest", "usage": {"input_tokens": 0, "output_tokens": 0}, "answers": {"bad": {"type": "unknown"}}}),
        json!({"model": "jev-latest", "usage": {"input_tokens": 0, "output_tokens": 0}, "answers": []}),
    ] {
        assert!(
            serde_json::from_value::<JevResponse>(payload.clone()).is_err(),
            "accepted {payload}"
        );
    }
}

#[test]
fn usage_constructor_and_safe_token_boundaries_round_trip() {
    for (input, output, total) in [
        (0, 0, 0),
        (12, 3, 15),
        (u64::MAX, 0, u64::MAX),
        (u64::MAX - 1, 1, u64::MAX),
    ] {
        let usage = Usage::as_budget(input, output);
        assert_eq!(usage.input_tokens(), input);
        assert_eq!(usage.output_tokens(), output);
        assert_eq!(usage.checked_tokens(), Some(total));
        assert_eq!(usage.tokens(), total);
        let payload = json!({"input_tokens": input, "output_tokens": output});
        assert_eq!(serde_json::to_value(&usage).unwrap(), payload);
        let decoded: Usage = serde_json::from_value(payload).unwrap();
        assert_eq!(decoded.tokens(), total);
    }
}

#[test]
fn usage_overflow_is_detectable_and_raw_counts_remain_available() {
    let usage = Usage::as_budget(u64::MAX, 1);
    assert_eq!(usage.checked_tokens(), None);
    assert_eq!(usage.tokens(), u64::MAX);
    assert_eq!(usage.input_tokens(), u64::MAX);
    assert_eq!(usage.output_tokens(), 1);
    assert!(usage.checked_add(&Usage::as_budget(1, 0)).is_none());
    let total = Usage::as_budget(2, 3)
        .checked_add(&Usage::as_budget(4, 5))
        .unwrap();
    assert_eq!((total.input_tokens(), total.output_tokens()), (6, 8));
}

#[test]
fn usage_rejects_missing_negative_fractional_and_string_counts() {
    for payload in [
        json!({"input_tokens": 0}),
        json!({"output_tokens": 0}),
        json!({"input_tokens": -1, "output_tokens": 0}),
        json!({"input_tokens": 0, "output_tokens": -1}),
        json!({"input_tokens": 0.5, "output_tokens": 0}),
        json!({"input_tokens": 0, "output_tokens": "1"}),
        json!({"input_tokens": null, "output_tokens": 0}),
    ] {
        assert!(
            serde_json::from_value::<Usage>(payload.clone()).is_err(),
            "accepted {payload}"
        );
    }
}

#[test]
fn usage_exceeds_budget_only_when_at_least_one_limit_is_exceeded() {
    let budget = Usage::as_budget(10, 20);
    // Exercise the Cartesian product of below, equal, and above both limits.
    for input in [9, 10, 11] {
        for output in [19, 20, 21] {
            let usage = Usage::as_budget(input, output);
            assert_eq!(
                usage.exceeds_budget(&budget),
                input > 10 || output > 20,
                "input={input}, output={output}"
            );
        }
    }
}

#[test]
fn usage_budget_comparison_handles_zero_and_maximum_counts_without_summing() {
    let zero = Usage::as_budget(0, 0);
    assert!(!zero.exceeds_budget(&zero));
    assert!(Usage::as_budget(1, 0).exceeds_budget(&zero));
    assert!(Usage::as_budget(0, 1).exceeds_budget(&zero));

    let maximum = Usage::as_budget(u64::MAX, u64::MAX);
    assert!(!maximum.exceeds_budget(&maximum));
    assert!(!zero.exceeds_budget(&maximum));
    assert!(maximum.exceeds_budget(&Usage::as_budget(u64::MAX - 1, u64::MAX)));
    assert!(maximum.exceeds_budget(&Usage::as_budget(u64::MAX, u64::MAX - 1)));
}

#[test]
fn tagged_variants_ignore_future_fields() {
    let question: Question = serde_json::from_value(json!({
        "type": "score", "instructions": "risk", "criteria": ["low"],
        "future_setting": {"enabled": true}
    }))
    .unwrap();
    assert_eq!(question, Question::score("risk", vec!["low"]));

    let answer: Answer = serde_json::from_value(json!({
        "type": "noul", "noul": 1.0, "future_metric": [1, 2]
    }))
    .unwrap();
    assert_eq!(answer, Answer::Noul { noul: 1.0 });
}
