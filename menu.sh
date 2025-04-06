#!/bin/bash
# RUSTYPROXY MANAGER

PORTS_FILE="/opt/rustyproxy/ports"

# Cores ANSI
RED="\033[1;31m"
GREEN="\033[1;32m"
YELLOW="\033[1;33m"
BLUE="\033[0;34m"
WHITE_BG="\033[40;1;37m"
RESET="\033[0m"

if [ "$EUID" -ne 0 ]; then
  echo -e "${RED}Por favor, execute este script como root ou com sudo.${RESET}"
  exit 1
fi

add_proxy_port() {
    local port=$1
    local status=${2:-"PROXY ANY"}

    if is_port_in_use "$port"; then
        echo -e "${RED}⛔️ A PORTA $port JÁ ESTÁ EM USO.${RESET}"
        return
    fi

    local command="/opt/rustyproxy/proxy --port $port --status \"$status\""
    local service_file_path="/etc/systemd/system/proxy${port}.service"
    local service_file_content="[Unit]
Description=RustyProxy ${port}
After=network.target

[Service]
LimitNOFILE=infinity
Type=simple
ExecStart=${command}
Restart=always

[Install]
WantedBy=multi-user.target"

    echo "$service_file_content" > "$service_file_path"
    systemctl daemon-reload
    systemctl enable "proxy${port}.service"
    systemctl start "proxy${port}.service"

    echo "$port|$status" >> "$PORTS_FILE"
    echo -e "${GREEN}✅ PORTA $port ABERTA COM SUCESSO.${RESET}"
}

is_port_in_use() {
    local port=$1
    if netstat -tuln 2>/dev/null | awk '{print $4}' | grep -q ":$port$"; then
        return 0
    elif ss -tuln 2>/dev/null | awk '{print $4}' | grep -q ":$port$"; then
        return 0
    elif lsof -i :"$port" 2>/dev/null | grep -q LISTEN; then
        return 0
    else
        return 1
    fi
}

del_proxy_port() {
    local port=$1

    systemctl disable "proxy${port}.service"
    systemctl stop "proxy${port}.service"
    rm -f "/etc/systemd/system/proxy${port}.service"
    systemctl daemon-reload

    if lsof -i :"$port" &>/dev/null; then
        fuser -k "$port"/tcp 2>/dev/null
    fi

    sed -i "/^$port|/d" "$PORTS_FILE"
    echo -e "${GREEN}✅ PORTA $port FECHADA COM SUCESSO.${RESET}"
}

update_proxy_status() {
    local port=$1
    local new_status=$2
    local service_file_path="/etc/systemd/system/proxy${port}.service"

    if ! is_port_in_use "$port"; then
        echo -e "${YELLOW}⚠️ A PORTA $port NÃO ESTÁ ATIVA.${RESET}"
        return
    fi

    if [ ! -f "$service_file_path" ]; then
        echo -e "${RED}ARQUIVO DE SERVIÇO PARA $port NÃO ENCONTRADO.${RESET}"
        return
    fi

    local new_command="/opt/rustyproxy/proxy --port $port --status \"$new_status\""
    sed -i "s|^ExecStart=.*$|ExecStart=${new_command}|" "$service_file_path"

    systemctl daemon-reload
    systemctl restart "proxy${port}.service"

    sed -i "s/^$port|.*/$port|$new_status/" "$PORTS_FILE"

    echo -e "${YELLOW}🔃 STATUS DA PORTA $port ATUALIZADO PARA '$new_status'.${RESET}"
    sleep 2
}

uninstall_rustyproxy() {
    echo -e "${YELLOW}🗑️ DESINSTALANDO RUSTY PROXY, AGUARDE...${RESET}"
    sleep 2
    clear

    if [ -s "$PORTS_FILE" ]; then
        while IFS='|' read -r port _; do
            del_proxy_port "$port"
        done < "$PORTS_FILE"
    fi

    rm -rf /opt/rustyproxy
    rm -f "$PORTS_FILE"

    echo -e "${BLUE}---------------------------------------------------------${RESET}"
    echo -e "${WHITE_BG}           RUSTY PROXY DESINSTALADO COM SUCESSO.          ${RESET}"
    echo -e "${BLUE}---------------------------------------------------------${RESET}"
    sleep 3
    clear
}

restart_all_proxies() {
    if [ ! -s "$PORTS_FILE" ]; then
        echo "NENHUMA PORTA ENCONTRADA PARA REINICIAR."
        return
    fi

    echo "🔃 REINICIANDO TODAS AS PORTAS DO PROXY..."
    sleep 2

    while IFS='|' read -r port status; do
        del_proxy_port "$port"
        add_proxy_port "$port" "$status"
    done < "$PORTS_FILE"

    echo -e "${GREEN}✅ TODAS AS PORTAS FORAM REINICIADAS COM SUCESSO.${RESET}"
    sleep 2
}

show_menu() {
    clear
    echo -e "${BLUE}--------------------------------------------------------------${RESET}"
    echo -e "${WHITE_BG}                  ⚒ RUSTY PROXY MANAGER ⚒                     ${RESET}"
    echo -e "${WHITE_BG}                        ${GREEN}VERSÃO: 0.2${RESET}                 "
    echo -e "${BLUE}--------------------------------------------------------------${RESET}"

    if [ ! -s "$PORTS_FILE" ]; then
        echo "NENHUMA PORTA ON"
    else
        while IFS='|' read -r port status; do
            echo -e " PORTA: ${YELLOW}$port${RESET} ON ${GREEN}$status${RESET}"
        done < "$PORTS_FILE"
    fi

    echo -e "${BLUE}--------------------------------------------------------------${RESET}"
    echo -e "${RED}[${CYAN}01${RED}] ${BLUE}◉ ${YELLOW}ATIVAR PROXY${RESET}"
    echo -e "${RED}[${CYAN}02${RED}] ${BLUE}◉ ${YELLOW}DESATIVAR PROXY${RESET}"
    echo -e "${RED}[${CYAN}03${RED}] ${BLUE}◉ ${YELLOW}REINICIAR PROXY${RESET}"
    echo -e "${RED}[${CYAN}04${RED}] ${BLUE}◉ ${YELLOW}ALTERAR STATUS${RESET}"
    echo -e "${RED}[${CYAN}05${RED}] ${BLUE}◉ ${YELLOW}REMOVER SCRIPT${RESET}"
    echo -e "${RED}[${CYAN}00${RED}] ${BLUE}◉ ${YELLOW}SAIR DO MENU${RESET}"
    echo -e "${BLUE}--------------------------------------------------------------${RESET}"

    read -p "  O QUE DESEJA FAZER ?: " option

    case $option in
        1)
            clear
            read -p "DIGITE A PORTA: " port
            while ! [[ $port =~ ^[0-9]+$ ]]; do
                echo "DIGITE UMA PORTA VÁLIDA."
                read -p "DIGITE A PORTA: " port
            done
            read -p "DIGITE O NOME DO STATUS: " status
            add_proxy_port "$port" "$status"
            read -n 1 -s -r -p "PRESSIONE QUALQUER TECLA PARA VOLTAR AO MENU."
            ;;
        2)
            clear
            read -p "DIGITE A PORTA: " port
            while ! [[ $port =~ ^[0-9]+$ ]]; do
                echo "DIGITE UMA PORTA VÁLIDA."
                read -p "DIGITE A PORTA: " port
            done
            del_proxy_port "$port"
            read -n 1 -s -r -p "PRESSIONE QUALQUER TECLA PARA VOLTAR AO MENU."
            ;;
        3)
            clear
            restart_all_proxies
            read -n 1 -s -r -p "PRESSIONE QUALQUER TECLA PARA VOLTAR AO MENU."
            ;;
        4)
            clear
            read -p "DIGITE A PORTA: " port
            while ! [[ $port =~ ^[0-9]+$ ]]; do
                echo "DIGITE UMA PORTA VÁLIDA."
                read -p "DIGITE A PORTA: " port
            done
            read -p "DIGITE O NOVO STATUS DO PROXY: " new_status
            update_proxy_status "$port" "$new_status"
            read -n 1 -s -r -p "PRESSIONE QUALQUER TECLA PARA VOLTAR AO MENU."
            ;;
        5)
            clear
            uninstall_rustyproxy
            read -n 1 -s -r -p "PRESSIONE QUALQUER TECLA PARA SAIR."
            clear
            exit 0
            ;;
        0)
            clear
            exit 0
            ;;
        *)
            echo "OPÇÃO INVÁLIDA. PRESSIONE QUALQUER TECLA PARA VOLTAR AO MENU."
            read -n 1 -s -r
            ;;
    esac
}

# Verifica ou cria o arquivo de controle de portas
[ ! -f "$PORTS_FILE" ] && touch "$PORTS_FILE"

# Loop do menu
while true; do
    show_menu
done
