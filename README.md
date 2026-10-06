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

#[tokio::main]
async fn main() {
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
        .unwrap();

    let resp = 
        TypeSafeClient::default()
        .system_one(&request)
        .await
        .unwrap();
    println!("{:#?}", resp);
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
公開の `client` フィールドを外部で差し替えた場合は, このポリシーを保証できません.

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
