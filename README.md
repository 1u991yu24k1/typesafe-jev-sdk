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
