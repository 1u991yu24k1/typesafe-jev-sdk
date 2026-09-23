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


