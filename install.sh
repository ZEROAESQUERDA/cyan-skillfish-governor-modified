#!/bin/bash

# Script de instalação para o Cyan Skillfish GPU Governor modificado

# Função para verificar se um comando existe
command_exists () {
  command -v "$1" >/dev/null 2>&1
}

# Verificar dependências
echo "Verificando dependências..."
if ! command_exists git; then
  echo "Erro: git não encontrado. Por favor, instale o git (sudo apt install git)."
  exit 1
fi

if ! command_exists cargo; then
  echo "Erro: cargo (Rust toolchain) não encontrado. Por favor, instale o Rust e o Cargo (https://rustup.rs/)."
  exit 1
fi

# Navegar para o diretório do projeto
PROJECT_DIR="$(dirname "$0")"
cd "$PROJECT_DIR" || {
  echo "Erro: Não foi possível navegar para o diretório do projeto."
  exit 1
}

echo "Compilando o governor..."
cargo build --release || {
  echo "Erro: Falha na compilação do projeto."
  exit 1
}

echo "Instalando o binário..."
sudo cp target/release/cyan-skillfish-governor-smu /usr/local/bin/ || {
  echo "Erro: Falha ao copiar o binário para /usr/local/bin."
  exit 1
}

echo "Configurando o arquivo de configuração e serviço systemd..."
sudo mkdir -p /etc/cyan-skillfish-governor/ || {
  echo "Erro: Falha ao criar o diretório /etc/cyan-skillfish-governor."
  exit 1
}
sudo cp custom-config.toml /etc/cyan-skillfish-governor/custom-config.toml || {
  echo "Erro: Falha ao copiar custom-config.toml."
  exit 1
}
sudo cp cyan-skillfish-governor.service /etc/systemd/system/ || {
  echo "Erro: Falha ao copiar o arquivo de serviço systemd."
  exit 1
}

echo "Recarregando o daemon systemd..."
sudo systemctl daemon-reload

echo "Habilitando e iniciando o serviço do governor..."
sudo systemctl enable cyan-skillfish-governor.service || {
  echo "Erro: Falha ao habilitar o serviço systemd."
  exit 1
}
sudo systemctl start cyan-skillfish-governor.service || {
  echo "Erro: Falha ao iniciar o serviço systemd."
  exit 1
}

echo "Instalação concluída com sucesso!"
echo "Para verificar o status do serviço, use: sudo systemctl status cyan-skillfish-governor.service"
echo "Para ver os logs em tempo real, use: journalctl -u cyan-skillfish-governor.service -f"
