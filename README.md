# typesafe-jev-sdk
TypeSafe AI Jev API Wrapper SDK 

## Install 
`Cargo.toml` に以下を記載. 
```toml
[dependencies]
typesafe-jev-sdk = { git = "https://github.com/1u991yu24k1/typesafe-jev-sdk.git", branch = "main" }
```

## Setup
以下の環境変数を記載. 
```shell
export TYPESAFE_API_BASE_URL="https://api.typesafe.ai/v1/systemone"
export TYPESAFE_API_KEY="apikey_....."

# Proxy Setting (Optional) 
export HTTPS_PROXY="https://...."
```

## Quick Start
```rust
use std::time::Duration;
use typesafe_jev_sdk::{builder::JevRequestBuilder, client::TypeSafeClient, JevError, Question};

#[tokio::main]
async fn main() -> Result<(), JevError> {
    let request = 
        JevRequestBuilder::new()
        .model("jev-latest")
        .state("プレイヤーのHPは20%。敵が近くに3体いる。\n回復アイテムを1個持っている。")
        .question(
            "next_action", 
            Question::choice(
                "次に取る行動は?",
                vec![
                    ("heal",    "回復アイテムを使ってHPを回復する"),
                    ("retreat", "敵から距離を取って退避する"),
                    ("attack",  "近くの敵を攻撃する")
                ]
            )
        )
        .build()
        ?;

    let resp = 
        TypeSafeClient::new(Duration::from_secs(5))?
        .system_one(&request)
        .await
        ?;
    // 応答をそのままログに出さず, 呼び出し側で処理する.
    let _usage = resp.usage;
    Ok(())
}
```


## Development

### Response metadata

`TypeSafeClient::system_one_with_metadata(&request)` は `JevResponseWithMetadata` を返します.
`response` は従来の `JevResponse`, `metadata` は HTTP の観測情報です.

- `metadata.status()`: 成功応答の HTTP ステータス.
- `metadata.headers()`: 応答ヘッダー. Cookie 等の機密情報を含み得るため, 無加工でログに出さないでください.
- `metadata.elapsed()`: リクエスト構築から本文受信と JSON デコードの完了までの時間. 呼び出し側での検証時間は含みません.
- `metadata.request_id(header_name)`: 明示指定したヘッダーの値. ヘッダーがない場合や文字列化できない場合は `None`.

JEV API の request ID ヘッダー名は未確認です. SDK はヘッダー名を推測せず, ID を生成しません.
このメソッドは自動再試行を行わず, 非 2xx, 通信失敗, 不正 JSON は従来と同じ `JevError` を返します.
失敗時の所要時間やヘッダーをエラーに追加する変更は含みません.
既存の `system_one` と API の JSON 形式は維持しています.
メタデータとメタデータ付き応答の `Debug` はヘッダー値と応答本文を伏せます.

### Retry policy

SDK が `try_new` / `new` で構築する HTTP クライアントは, 自動再試行と自動リダイレクトを無効にしています.
reqwest 0.13.5 は既定で protocol NACK を再試行しますが, SDK は `retry::never()` を明示指定します.
307/308 による POST 本文の再送も避けるため, `redirect::Policy::none()` を指定します.
3xx は既存の非 2xx ポリシーに従って `JevError::HttpStatus` になります.
`client` と `url` は非公開で, 構築後に通信ポリシーを差し替えることはできません.

2026-10-06 に確認した [公開 OpenAPI 0.2.0](https://api.typesafe.ai/openapi.json) と
[ReDoc](https://api.typesafe.ai/redoc) は, POST `/v1/systemone` と usage を定義しています.
確認した仕様には以下の契約が見当たらず, 再送が安全とは判断していません.

- 冪等性キーの有無, 有効期間, 同じキーと異なる本文を送った場合の扱い.
- timeout, 切断, 429, 5xx の際の課金と, 同じ POST を再送した場合の重複課金の扱い.
- `Retry-After` の返却条件, 待機規則, 再試行上限.

未記載はサービス側に機能が存在しないという意味ではありません. 契約の確認が必要です.
`JevError::retryable()` は 429/5xx というステータス分類のヒントであり, 再送の許可を意味しません.
timeout や切断から, サーバーが処理していないとは判断できません.
`Retry-After` があっても SDK は待機や再送を行わず, 試行回数や課金を推定しません.

自動再試行を追加する場合は, サービス側の保証を確認した後に, 明示的な opt-in,
回数と総時間の上限, 待機と jitter, `Retry-After` の上限付き処理,
冪等性キーの再利用, 試行記録を別課題として設計します.
この確認のために推論 API を呼ぶ必要はありません. ライブテストは別途承認後に実施します.

### Local validation

通常のテストは API キーや外部 API 接続を必要としません。クライアントの通信は
ローカルの一時 HTTP サーバーで検証します。

```sh
cargo test --locked
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
```

ベンチマークはリクエスト構築処理のみを測定します。実 API の応答時間を測るものではありません。

```sh
cargo bench --locked --bench builder
cargo bench --locked --bench allocations
```

カバレッジ計測には `cargo-llvm-cov` と `llvm-tools-preview` が必要です。

```sh
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
cargo llvm-cov --locked --html
```

`Usage::exceeds_budget` は入力・出力のいずれかが対応する上限を **超えた** 場合に
`true` を返します。上限と同じ値は超過とは見なしません。

`Usage::input_tokens()` と `Usage::output_tokens()` は API が返した生の値を返します.
厳密な合計と複数応答の累積には `checked_tokens()` と `checked_add()` を使用してください.
互換 API の `tokens()` はオーバーフロー時に `u64::MAX` に飽和します.

### Configuration and migration

今回の変更はコンストラクターとフィールド公開範囲に破壊的変更を含みます.

- `TypeSafeClient::new(timeout)` は `Result<TypeSafeClient, JevError>` を返します. `?` 等で設定エラーを処理してください.
- panic を避けるため `TypeSafeClient::default()` は削除しました. `TypeSafeClient::new(Duration::from_secs(5))?` に置き換えてください.
- `client` / `url` は非公開です. URL の参照には `endpoint()` を使います. 機密 query を含み得るため, 無加工でログに出さないでください.
- 個別設定には `TypeSafeClient::try_new(Config::new(endpoint, key)?.connect_timeout(...).request_timeout(...))` を使います.
- `TYPESAFE_API_BASE_URL` は完全な endpoint です. 非 HTTPS はクライアント構築時に拒否します. ローカルモックでは `Config::allow_http_for_testing()` を明示指定します.
- userinfo / fragment を含む endpoint, 空の API キー, bearer ヘッダーに不適切な文字, ゼロの timeout は拒否します.
- 接続 timeout の既定値は 5 秒, 全体 timeout は 30 秒です. `new(timeout)` の引数は接続 timeout のみを変更します.

### Strict validation

`JevRequestBuilder::strict_validation()` と `JevRequest::validate_strict()` は任意の検証です.
既存の `choice` / `noul` コンストラクターと既定 Builder は引き続き permissive です.
`Question::try_choice` / `try_noul` は map 化の前に候補 ID の重複を検出します.
既に map 化された候補からは, 上書き前の重複を復元できません.
厳格な候補 ID は空文字, 空白, 制御文字を拒否しますが, Unicode は許可します.
これは SDK の opt-in 方針であり, API の許容文字を推定したものではありません.
Noul の criteria は省略できますが, 厳格検証では明示的な空候補を拒否します.

応答の意味検証は `response.validate_against(&request)` で行います.
生の JSON デシリアライズとは分離されており, 自動適用しません.
`JevError` の `Debug` / `Display` は HTTP 本文と下位エラーの文字列を伏せます.
`error_body()` は最大 1024 バイトの受信断片から API キーを伏せた本文です.
他の機密値を自動判定する機能ではないため, 無加工で保存やログ出力をしないでください.
