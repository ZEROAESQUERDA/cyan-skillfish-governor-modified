# Modificações no Governor `cyan-skillfish-governor`

Este documento detalha as modificações implementadas no governor `cyan-skillfish-governor` (branch `smu`) para adicionar um sistema de boost escalonável com controle térmico, incluindo a capacidade de operar a uma frequência mínima de 1000 MHz.

## 1. Visão Geral das Mudanças

As principais alterações foram realizadas no arquivo `src/main.rs` para introduzir uma lógica de ajuste de frequência da GPU baseada na carga e na temperatura. Um novo arquivo de configuração, `custom-config.toml`, foi criado e atualizado para otimizar os parâmetros do governor para o novo comportamento.

## 2. Lógica de Boost Escalonável

O sistema de boost agora opera com os seguintes níveis de frequência:

*   **Frequência Mínima:** 1000 MHz (atingida em cenários de carga muito baixa/idle).
*   **Frequência Base:** 1500 MHz (ativada sob carga normal).
*   **Boost Médio:** 1750 MHz (ativado sob carga moderada).
*   **Boost Máximo:** 2000 MHz (ativado sob carga alta ou burst).

A decisão sobre qual frequência alvo aplicar é feita com base na `average_load` (carga média da GPU) e na detecção de `burst` (picos de carga). Os thresholds de carga (`up_thresh` e `down_thresh`) definidos no arquivo de configuração são utilizados para determinar a transição entre esses estados. Especificamente, uma carga muito baixa (abaixo de `config.down_thresh`) fará com que a frequência caia para 1000 MHz.

## 3. Controle Térmico Dinâmico

Foi implementado um mecanismo de controle térmico para proteger a GPU de superaquecimento e gerenciar a frequência de forma adaptativa:

*   **Limite de Temperatura:** 70°C.
*   **Redução de Clock:** Se a temperatura da GPU exceder 70°C, a frequência alvo será reduzida em incrementos de 50 MHz. Essa redução é acumulativa e armazenada na variável `thermal_throttle_offset`.
*   **Recuperação de Clock:** Quando a temperatura da GPU retornar a 70°C ou menos, o governor aguardará um período de 10 segundos (`recovery_delay`). Após esse período, a frequência começará a subir gradualmente, reduzindo o `thermal_throttle_offset` em 50 MHz por vez, até que o offset seja zero ou a temperatura volte a subir.
*   **Frequência Mínima Absoluta:** A frequência nunca será reduzida abaixo da frequência mínima de 1000 MHz devido ao controle térmico.

## 4. Detalhes da Implementação (`src/main.rs`)

As seguintes variáveis de estado foram adicionadas/atualizadas:

*   `min_freq`, `base_freq`, `mid_boost_freq`, `max_boost_freq`: Constantes para as frequências definidas.
*   `thermal_throttle_offset`: Armazena a redução de frequência aplicada devido ao thermal throttling.
*   `last_over_temp_time`: Registra o `Instant` em que a temperatura excedeu 70°C pela última vez, usado para o delay de recuperação.
*   `recovery_delay`: Constante `Duration` de 10 segundos para o tempo de espera antes de subir o clock.

A lógica principal no loop do governor foi modificada para:

1.  Ler a temperatura da GPU (`gpu.read_temperature()?`).
2.  Aplicar ou reduzir o `thermal_throttle_offset` com base na temperatura e no `recovery_delay`.
3.  Calcular uma `load_target_freq` (frequência alvo baseada na carga) usando os thresholds de carga e as frequências de boost (1000, 1500, 1750, 2000 MHz).
4.  Subtrair o `thermal_throttle_offset` da `load_target_freq` para obter a `target_freq` final, garantindo que não seja menor que `min_freq`.
5.  Aplicar a `target_freq` se houver uma `big_change` (mudança significativa, definida como 25 MHz para evitar oscilações). Um `println!` foi adicionado para logar os ajustes de frequência, temperatura, carga e offset.

## 5. Arquivo de Configuração (`custom-config.toml`)

O arquivo de configuração `custom-config.toml` foi atualizado para refletir a nova frequência mínima e os `safe-points` correspondentes:

```toml
# Configuração otimizada para Boost Escalonável e Controle Térmico

[timing.intervals]
sample = 2000    # 2ms por amostra
adjust = 200000  # 200ms por ciclo de ajuste

[timing.ramp-rates]
normal = 1.0
burst = 50.0

[timing]
burst-samples = 48
down-events = 5

[frequency-thresholds]
adjust = 25

[load-target]
upper = 0.85
lower = 0.40 # Reduzido para permitir cair para 1000MHz em idle/baixa carga

[temperature]
throttling = 70
throttling_recovery = 70

# Safe points cobrindo as frequências de interesse (1000 até 2000)
[[safe-points]]
frequency = 1000
voltage = 800

[[safe-points]]
frequency = 1100
voltage = 820

[[safe-points]]
frequency = 1200
voltage = 840

[[safe-points]]
frequency = 1300
voltage = 860

[[safe-points]]
frequency = 1400
voltage = 880

[[safe-points]]
frequency = 1500
voltage = 900

[[safe-points]]
frequency = 1600
voltage = 910

[[safe-points]]
frequency = 1700
voltage = 920

[[safe-points]]
frequency = 1750
voltage = 925

[[safe-points]]
frequency = 1800
voltage = 930

[[safe-points]]
frequency = 1850
voltage = 935

[[safe-points]]
frequency = 1900
voltage = 940

[[safe-points]]
frequency = 1950
voltage = 950

[[safe-points]]
frequency = 2000
voltage = 960
```

**Observações sobre a configuração:**

*   Os `safe-points` foram expandidos para incluir as frequências de 1000 MHz até 2000 MHz, com voltagens de exemplo. É crucial que esses valores sejam validados para a sua GPU específica para evitar instabilidade.
*   O `load-target.lower` foi ajustado para `0.40` para permitir que a frequência caia para 1000 MHz em cenários de carga muito baixa.

## 6. Como Usar

Para compilar e executar o governor com as novas modificações e a configuração personalizada:

1.  **Navegue até o diretório do projeto:**
    ```bash
    cd /home/ubuntu/cyan-skillfish-governor-modified
    ```
2.  **Compile o projeto:**
    ```bash
    cargo build --release
    ```
3.  **Execute o governor com a configuração personalizada:**
    ```bash
    sudo ./target/release/cyan-skillfish-governor-smu custom-config.toml
    ```

## 7. Como Instalar como um Serviço `systemd`

Para que o governor inicie automaticamente com o sistema e rode em segundo plano, você pode configurá-lo como um serviço `systemd`:

1.  **Copie o binário executável para um local acessível pelo sistema:**
    ```bash
    sudo cp target/release/cyan-skillfish-governor-smu /usr/local/bin/
    ```
2.  **Crie um diretório para a configuração personalizada:**
    ```bash
    sudo mkdir -p /etc/cyan-skillfish-governor/
    ```
3.  **Copie o arquivo de configuração personalizado para o novo diretório:**
    ```bash
    sudo cp custom-config.toml /etc/cyan-skillfish-governor/custom-config.toml
    ```
4.  **Copie o arquivo de serviço `systemd` para o diretório de serviços do sistema:**
    ```bash
    sudo cp cyan-skillfish-governor.service /etc/systemd/system/
    ```
5.  **Recarregue o `systemd` para reconhecer o novo serviço:**
    ```bash
    sudo systemctl daemon-reload
    ```
6.  **Habilite o serviço para iniciar automaticamente na inicialização do sistema:**
    ```bash
    sudo systemctl enable cyan-skillfish-governor.service
    ```
7.  **Inicie o serviço agora:**
    ```bash
    sudo systemctl start cyan-skillfish-governor.service
    ```

Para verificar o status do serviço e ver os logs:
```bash
sudo systemctl status cyan-skillfish-governor.service
journalctl -u cyan-skillfish-governor.service -f
```

**Importante:** A execução do governor requer privilégios de root (`sudo`). Certifique-se de entender os riscos associados à modificação de frequências e voltagens da GPU. Recomenda-se monitorar a estabilidade e a temperatura da GPU durante o uso.
