#!/bin/bash
# MENU RUSTYPROXY COM SSLH

PORTS_FILE="/opt/rustyproxy/ports"
SSLH_CONFIG="/etc/sslh/sslh.conf"

# Função para instalar o SSLH
install_sslh() {
    echo "Instalando SSLH..."
    sudo apt update && sudo apt install -y sslh
    echo "SSLH instalado com sucesso."
}

# Função para configurar o SSLH
configure_sslh() {
    echo "Configurando SSLH..."
    sudo bash -c "cat > $SSLH_CONFIG" <<EOL
verbose: true;
foreground: true;
inetd: false;
numeric: true;

listen:
(
    { host: "0.0.0.0"; port: "443"; }
);

protocols:
(
    { name: "ssh"; host: "127.0.0.1"; port: "22"; probe: "ssh"; },
    { name: "https"; host: "127.0.0.1"; port: "8443"; probe: "tls"; },
    { name: "fallback"; host: "127.0.0.1"; port: "8080"; }
);
EOL
    sudo systemctl restart sslh
    echo "SSLH configurado e reiniciado."
}

# Função para iniciar o SSLH
start_sslh() {
    sudo systemctl enable sslh
    sudo systemctl start sslh
    echo "SSLH iniciado."
}

# Função para parar o SSLH
stop_sslh() {
    sudo systemctl stop sslh
    echo "SSLH parado."
}

# Função para remover o SSLH
remove_sslh() {
    sudo systemctl stop sslh
    sudo apt remove --purge -y sslh
    echo "SSLH removido."
}

# Menu principal
show_menu() {
    clear
    echo "MENU RUSTY PROXY COM SSLH"
    echo "1) Instalar SSLH"
    echo "2) Configurar SSLH"
    echo "3) Iniciar SSLH"
    echo "4) Parar SSLH"
    echo "5) Remover SSLH"
    echo "0) Sair"
    read -p "Escolha uma opção: " option

    case $option in
        1) install_sslh ;;
        2) configure_sslh ;;
        3) start_sslh ;;
        4) stop_sslh ;;
        5) remove_sslh ;;
        0) exit 0 ;;
        *) echo "Opção inválida." ;;
    esac
}

while true; do
    show_menu
    read -p "Pressione ENTER para continuar..." dummy
done