#!/bin/bash
# Script para configurar e gerenciar Rusty Proxy, Squid e HAProxy automaticamente

# Variáveis
PORT_HAPROXY=8080
PORT_SQUID=3129
PORT_RUSTY=3130
HAPROXY_CFG="/etc/haproxy/haproxy.cfg"
PORTS_FILE="/opt/rustyproxy/ports"

# Instalar dependências
install_dependencies() {
    echo "Instalando HAProxy, Squid e Rusty Proxy..."
    sudo apt update
    sudo apt install -y haproxy squid
}

# Configurar HAProxy
configure_haproxy() {
    echo "Configurando HAProxy..."
    sudo bash -c "cat > $HAPROXY_CFG" <<EOF
global
    log /dev/log local0
    log /dev/log local1 notice
    chroot /var/lib/haproxy
    stats socket /run/haproxy/admin.sock mode 660 level admin
    stats timeout 30s
    user haproxy
    group haproxy
    daemon

defaults
    log global
    mode tcp
    option tcplog
    timeout connect 5000ms
    timeout client 50000ms
    timeout server 50000ms

frontend proxy_frontend
    bind *:$PORT_HAPROXY
    mode tcp
    default_backend proxy_backend

backend proxy_backend
    mode tcp
    balance roundrobin
    server squid_proxy 127.0.0.1:$PORT_SQUID check
    server rusty_proxy 127.0.0.1:$PORT_RUSTY check
EOF
    sudo systemctl restart haproxy
}

# Configurar Squid
configure_squid() {
    echo "Configurando Squid na porta $PORT_SQUID..."
    sudo sed -i "s/^http_port .*/http_port $PORT_SQUID/" /etc/squid/squid.conf
    sudo systemctl restart squid
}

# Configurar Rusty Proxy
add_rustyproxy_service() {
    local port=$PORT_RUSTY
    local command="/opt/rustyproxy/proxy --port $port"
    local service_file="/etc/systemd/system/rustyproxy.service"

    echo "Criando serviço para Rusty Proxy na porta $port..."
    sudo bash -c "cat > $service_file" <<EOF
[Unit]
Description=RustyProxy $port
After=network.target

[Service]
LimitNOFILE=infinity
Type=simple
ExecStart=$command
Restart=always

[Install]
WantedBy=multi-user.target
EOF

    sudo systemctl daemon-reload
    sudo systemctl enable rustyproxy.service
    sudo systemctl start rustyproxy.service
}

# Menu interativo
menu() {
    while true; do
        clear
        echo "==========================="
        echo "  Gerenciador de Proxies  "
        echo "==========================="
        echo "1) Instalar dependências"
        echo "2) Configurar Squid"
        echo "3) Configurar Rusty Proxy"
        echo "4) Configurar HAProxy"
        echo "5) Reiniciar Squid"
        echo "6) Reiniciar Rusty Proxy"
        echo "7) Reiniciar HAProxy"
        echo "8) Sair"
        echo "==========================="
        read -p "Escolha uma opção: " opcao

        case $opcao in
            1) install_dependencies;;
            2) configure_squid;;
            3) add_rustyproxy_service;;
            4) configure_haproxy;;
            5) sudo systemctl restart squid; echo "Squid reiniciado!"; sleep 2;;
            6) sudo systemctl restart rustyproxy; echo "Rusty Proxy reiniciado!"; sleep 2;;
            7) sudo systemctl restart haproxy; echo "HAProxy reiniciado!"; sleep 2;;
            8) exit;;
            *) echo "Opção inválida!"; sleep 2;;
        esac
    done
}

# Executar menu
menu
