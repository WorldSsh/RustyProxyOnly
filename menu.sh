#!/bin/bash

MENU RUSTYPROXY

PORTS_FILE="/opt/rustyproxy/ports"

Verificar se o script está sendo executado como root

if [[ $EUID -ne 0 ]]; then echo "⚠️ Este script deve ser executado como root." >&2 exit 1 fi

Função para verificar se uma porta está em uso

is_port_in_use() { local port=$1 ss -tuln | awk '{print $4}' | grep -q ":$port$" }

Função para abrir uma porta proxy

add_proxy_port() { local port=$1 local status=${2:-"@RustyProxy"}

if is_port_in_use $port; then
    echo "⛔ A PORTA $port JÁ ESTÁ EM USO."
    return
fi

local command="/opt/rustyproxy/proxy --port $port --status \"$status\""
local service_file_path="/etc/systemd/system/proxy${port}.service"

cat <<EOF | sudo tee "$service_file_path" > /dev/null

[Unit] Description=RustyProxy ${port} After=network.target

[Service] LimitNOFILE=infinity Type=simple ExecStart=${command} Restart=always

[Install] WantedBy=multi-user.target EOF

sudo systemctl daemon-reload
sudo systemctl enable "proxy${port}.service"
sudo systemctl start "proxy${port}.service"

echo "$port $status" >> "$PORTS_FILE"
echo "✅ Porta $port ABERTA COM SUCESSO."

}

Função para fechar uma porta proxy

del_proxy_port() { local port=$1

sudo systemctl disable "proxy${port}.service"
sudo systemctl stop "proxy${port}.service"
sudo rm -f "/etc/systemd/system/proxy${port}.service"
sudo systemctl daemon-reload

fuser -k "$port"/tcp 2>/dev/null
sed -i "/^$port /d" "$PORTS_FILE"
echo "✅ Porta $port FECHADA COM SUCESSO."

}

Função para reiniciar todas as portas proxies abertas

restart_all_proxies() { if [ ! -s "$PORTS_FILE" ]; then echo "⚠️ NENHUMA PORTA ENCONTRADA PARA REINICIAR." return fi

echo "🔄 REINICIANDO TODAS AS PORTAS..."
while read -r line; do
    port=$(echo "$line" | awk '{print $1}')
    status=$(echo "$line" | cut -d' ' -f2-)
    del_proxy_port "$port"
    add_proxy_port "$port" "$status"
done < "$PORTS_FILE"

echo "✅ TODAS AS PORTAS FORAM REINICIADAS COM SUCESSO."

}

Função para remover o script do sistema

remove_script() { echo "⚠️ REMOVENDO O SCRIPT..." rm -f "$0" echo "✅ SCRIPT REMOVIDO COM SUCESSO!" exit 0 }

Função para exibir o menu

show_menu() { clear echo "--------------------------------------------------------------" echo "                  ⚒ RUSTY PROXY MANAGER ⚒                     " echo "                        VERSÃO: 02                            " echo "--------------------------------------------------------------"

if [ ! -s "$PORTS_FILE" ]; then
    echo "🚫 NENHUMA PORTA ATIVA"
else
    while read -r line; do
        port=$(echo "$line" | awk '{print $1}')
        status=$(echo "$line" | cut -d' ' -f2-)
        printf " 🌐 PORTA: %-5s ON %s\n" "$port" "$status"
    done < "$PORTS_FILE"
fi

echo "--------------------------------------------------------------"
echo "[1] 🟢 ABRIR PORTAS"
echo "[2] 🔴 FECHAR PORTAS"
echo "[3] 🔄 REINICIAR PORTAS"
echo "[4] ❌ REMOVER SCRIPT"
echo "[0] ⏹ SAIR"
echo "--------------------------------------------------------------"
read -p "  O QUE DESEJA FAZER ?: " option

case $option in
    1)
        read -p "DIGITE A PORTA: " port
        while ! [[ "$port" =~ ^[0-9]+$ ]]; do
            echo "Erro: Digite uma porta válida."
            read -p "DIGITE A PORTA: " port
        done
        read -p "DIGITE O NOME DO STATUS: " status
        add_proxy_port $port "$status"
        ;;
    2)
        read -p "DIGITE A PORTA: " port
        while ! [[ "$port" =~ ^[0-9]+$ ]]; do
            echo "Erro: Digite uma porta válida."
            read -p "DIGITE A PORTA: " port
        done
        del_proxy_port $port
        ;;
    3)
        restart_all_proxies
        ;;
    4)
        remove_script
        ;;
    0)
        exit 0
        ;;
    *)
        echo "⚠️ OPÇÃO INVÁLIDA."
        ;;
esac

}

Verificar se o arquivo de portas existe

if [ ! -f "$PORTS_FILE" ]; then sudo touch "$PORTS_FILE" fi

Loop do menu

while true; do show_menu done

