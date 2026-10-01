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
