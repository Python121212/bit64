use axum::{routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::services::ServeDir;

// CPUの状態を定義する構造体
#[derive(Serialize, Deserialize, Clone)]
struct CpuState {
    eax: u32,
    ebx: u32,
    eip: u32,
    memory: Vec<u8>,
    log: String,
}

// フロントから送られてくる実行リクエスト
#[derive(Deserialize)]
struct StepRequest {
    cpu: CpuState,
}

#[tokio::main]
async fn main() {
    // フロントエンドの静的ファイル(publicフォルダ)を配信し、APIルートを設定
    let app = Router::new()
        .nest_service("/", ServeDir::new("public"))
        .route("/api/step", post(run_cpu_step));

    // Renderの環境変数 PORT に対応（デフォルトは3000）
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap();
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("サーバー起動中: http://localhost:{}", port);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Fetch -> Decode -> Execute を行うAPI
async fn run_cpu_step(Json(payload): Json<StepRequest>) -> Json<CpuState> {
    let mut cpu = payload.cpu;
    let eip = cpu.eip as usize;

    if eip >= cpu.memory.len() {
        cpu.log = "エラー: EIPがメモリの範囲を超えました。".to_string();
        return Json(cpu);
    }

    // 【Fetch】メモリから1バイト読み込む
    let opcode = cpu.memory[eip];
    cpu.eip += 1;

    // 【Decode & Execute】
    match opcode {
        0xB8 => { // MOV EAX, imm32 (4バイトの数値をEAXに代入)
            let val = read_mem_32(&cpu.memory, cpu.eip as usize);
            cpu.eax = val;
            cpu.eip += 4;
            cpu.log = format!("Executed: MOV EAX, {}", val);
        }
        0xBB => { // MOV EBX, imm32 (4バイトの数値をEBXに代入)
            let val = read_mem_32(&cpu.memory, cpu.eip as usize);
            cpu.ebx = val;
            cpu.eip += 4;
            cpu.log = format!("Executed: MOV EBX, {}", val);
        }
        0x01 => { // ADD EAX, EBX (EAXにEBXを足す)
            cpu.eax = cpu.eax.wrapping_add(cpu.ebx);
            cpu.log = "Executed: ADD EAX, EBX".to_string();
        }
        0x00 => { // NOP またはプログラムの終端
            cpu.log = "End of Code (NOP)".to_string();
        }
        _ => {
            cpu.log = format!("未実装の命令コードです: 0x{:02X}", opcode);
        }
    }

    Json(cpu)
}

// 4バイト分をまとめてu32として読み出すヘルパー
fn read_mem_32(mem: &[u8], start: usize) -> u32 {
    if start + 4 > mem.len() { return 0; }
    u32::from_le_bytes([mem[start], mem[start+1], mem[start+2], mem[start+3]])
}
