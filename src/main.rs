mod config;
mod gpu;
use config::Config;
use gpu::GPU;
use std::time::{Instant, Duration};
use std::fs::OpenOptions;
use std::io::Write;

fn log_to_file(message: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let log_message = format!("[{}] {}\n", timestamp, message);
    
    // Tenta escrever no console
    print!("{}", log_message);
    
    // Tenta escrever no arquivo de log
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/var/log/cyan-skillfish-governor.log") {
            let _ = file.write_all(log_message.as_bytes());
            let _ = file.flush();
        }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::new(
        std::env::args()
            .nth(1)
            .map(std::fs::read_to_string)
            .unwrap_or(Ok("".to_string())),
    )?;

    let mut gpu = GPU::new(config.safe_points)?;

    let mut curr_freq: u32 = gpu.get_freq()?;
    let mut curr_vol: u32 = 0; // Será atualizado na primeira mudança
    
    // Definições de frequências solicitadas
    let min_freq: u32 = 1000;
    let base_freq: u32 = 1500;
    let mid_boost_freq: u32 = 1750;
    let max_boost_freq: u32 = 2000;
    
    let mut target_freq = base_freq;
    
    // Inicializa na frequência base
    curr_vol = gpu.change_freq(base_freq)?;
    
    // Variáveis para controle térmico
    let mut thermal_throttle_offset: u32 = 0;
    let mut last_over_temp_time: Option<Instant> = None;
    let target_temp: u32 = 70;
    let recovery_delay = Duration::from_secs(10);

    log_to_file("=== Governor Iniciado com Logs Detalhados ===");
    log_to_file(&format!("Frequências: Mín: {}MHz, Base: {}MHz, Mid: {}MHz, Max: {}MHz", min_freq, base_freq, mid_boost_freq, max_boost_freq));

    loop {
        let mut average_load: f32 = 0.0;
        let mut burst_length: u32 = 0;

        // Amostragem
        for _ in 0..65 {
            (average_load, burst_length) = gpu.poll_and_get_load()?;
            std::thread::sleep(config.sampling_interval);
        }

        let burst = config
            .burst_samples
            .map_or(false, |burst_samples| burst_length >= burst_samples);

        // --- Lógica de Controle Térmico ---
        let temp = gpu.read_temperature()?;
        
        if temp > target_temp {
            thermal_throttle_offset += 50;
            last_over_temp_time = Some(Instant::now());
            log_to_file(&format!("ALERTA TÉRMICO: {}°C. Throttle aumentado para: -{}MHz", temp, thermal_throttle_offset));
        } else if thermal_throttle_offset > 0 {
            if let Some(last_time) = last_over_temp_time {
                if Instant::now().duration_since(last_time) >= recovery_delay {
                    if thermal_throttle_offset >= 50 {
                        thermal_throttle_offset -= 50;
                    } else {
                        thermal_throttle_offset = 0;
                    }
                    last_over_temp_time = Some(Instant::now());
                    log_to_file(&format!("RECUPERAÇÃO TÉRMICA: {}°C. Throttle reduzido para: -{}MHz", temp, thermal_throttle_offset));
                }
            }
        }

        // --- Lógica de Decisão de Frequência baseada em Carga ---
        let mut load_target_freq;
        let mut reason = "carga";
        
        if burst {
            load_target_freq = max_boost_freq;
            reason = "burst";
        } else if average_load > config.up_thresh {
            load_target_freq = max_boost_freq;
            reason = "carga alta";
        } else if average_load > (config.up_thresh + config.down_thresh) / 2.0 {
            load_target_freq = mid_boost_freq;
            reason = "carga média";
        } else if average_load > config.down_thresh {
            load_target_freq = base_freq;
            reason = "carga normal";
        } else {
            load_target_freq = min_freq;
            reason = "carga baixa";
        }

        // Aplicar o throttle térmico
        if load_target_freq > thermal_throttle_offset {
            target_freq = load_target_freq - thermal_throttle_offset;
        } else {
            target_freq = min_freq;
        }

        target_freq = target_freq.clamp(min_freq, max_boost_freq);
        
        // Histerese: Apenas mudar se a diferença for significativa
        let big_change = curr_freq.abs_diff(target_freq) >= 25; 

        if curr_freq != target_freq && big_change {
            let old_freq = curr_freq;
            let old_vol = curr_vol;
            
            match gpu.change_freq(target_freq) {
                Ok(new_vol) => {
                    curr_freq = target_freq;
                    curr_vol = new_vol;
                    log_to_file(&format!(
                        "MUDANÇA: {}MHz@{}mV -> {}MHz@{}mV | Motivo: {} | Temp: {}°C | Carga: {:.2} | Offset: -{}MHz",
                        old_freq, old_vol, curr_freq, curr_vol, reason, temp, average_load, thermal_throttle_offset
                    ));
                },
                Err(e) => {
                    log_to_file(&format!("ERRO CRÍTICO ao mudar frequência: {}", e));
                }
            }
        }

        std::thread::sleep(config.adjustment_interval.saturating_sub(64 * config.sampling_interval));
    }
}
