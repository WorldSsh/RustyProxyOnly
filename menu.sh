#!/bin/bash
#MENU RUSTYPROXY

PORTS_FILE="/opt/rustyproxy/ports"

#FUNÇÃO PARA ABRIR PORTAS DE UM PROXY
add_proxy_port() {
    local port=$1
    local status=${2:-"@RustyProxy"}

    if is_port_in_use $port; then
        echo "A PORTA $port JÁ ESTA EM USO."
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

    echo "$service_file_content" | sudo tee "$service_file_path" > /dev/null
    sudo systemctl daemon-reload
    sudo systemctl enable "proxy${port}.service"
    sudo systemctl start "proxy${port}.service"

    # Salvar a porta no arquivo
    echo "$port \033[1;31m$status\033[0m" >> "$PORTS_FILE"
    echo "Porta $port ABERTA COM SUCESSO."
	clear
}

#FUNÇÃO VERIFICAR PORTAS EM USO
is_port_in_use() {
    local port=$1

    # Verifica conexões ESTABELECIDAS ou LISTEN
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

#FUNÇÃO PARA FECHAR PORTAS DE UM PROXY
del_proxy_port() {
    local port=$1

    sudo systemctl disable "proxy${port}.service"
    sudo systemctl stop "proxy${port}.service"
    sudo rm -f "/etc/systemd/system/proxy${port}.service"
    sudo systemctl daemon-reload

    # Matar qualquer processo que ainda esteja usando a porta
    fuser -k "$port"/tcp 2>/dev/null

    # Remover a porta do arquivo de controle
    sed -i "/^$port /d" "$PORTS_FILE"
    echo "Porta $port FECHADA COM SUCESSO."
    clear
}

#FUNÇÃO PARA ALTERAR UM STATUS DE UM PROXY
update_proxy_status() {
    local port=$1
    local new_status=$2
    local service_file_path="/etc/systemd/system/proxy${port}.service"

    if ! is_port_in_use $port; then
        echo "A PORTA $port NÃO ESTÁ ATIVA."
        return
    fi

    if [ ! -f "$service_file_path" ]; then
        echo "ARQUIVO DE SERVIÇO PARA $port NÃO ENCONTRADO."
        return
    fi

    local new_command="/opt/rustyproxy/proxy --port $port --status \"$new_status\""
    sudo sed -i "s|^ExecStart=.*$|ExecStart=${new_command}|" "$service_file_path"

    sudo systemctl daemon-reload
    sudo systemctl restart "proxy${port}.service"

    # Atualizar o arquivo de portas
    sed -i "s/^$port .*/$port $new_status/" "$PORTS_FILE"

    echo "STATUS DA PORTA $port ATUALIZADO PARA '$new_status'."
    sleep 3
    clear
}

#FUNÇÃO PARA DESINSTALAR RUSTY PROXY
    uninstall_rustyproxy() {
    echo "DESINSTALANDO RUSTY PROXY, AGUARDE..."
    sleep 4
    clear

#REMOVER TODOS OS SERVIÇOS
    if [ -s "$PORTS_FILE" ]; then
        while read -r port; do
            del_proxy_port $port
        done < "$PORTS_FILE"
    fi
	
	#REMOVER BINÁRIOS, ARQUIVOS E DIRETÓRIOS
    sudo rm -rf /opt/rustyproxy
    sudo rm -f "$PORTS_FILE"

    echo -e "\033[0;34m---------------------------------------------------------\033[0m"
    echo -e "\E[44;1;37m           RUSTY PROXY DESINSTALADO COM SUCESSO.          \E[0m"
    echo -e "\033[0;34m---------------------------------------------------------\033[0m"
    sleep 4
    clear
}

#EXIBIR MENU
show_menu() {
    clear
    echo -e "\033[0;34m--------------------------------------------------------------\033[0m"
    echo -e "\033[40;1;37m                  ⚒ RUSTY PROXY MANAGER ⚒                    \E[0m"
    echo -e "\033[40;1;37m                        \033[1;32mVERSÃO: 02                             "
    echo -e "\033[0;34m--------------------------------------------------------------\033[0m"

   # Verifica se há portas ativas
    if [ ! -s "$PORTS_FILE" ]; then
        printf " PORTA ATIVA(s): %-34s\n" "NENHUMA"
    else
        while read -r line; do
            port=$(echo "$line" | awk '{print $1}')
            status=$(echo "$line" | cut -d' ' -f2-)
            printf " PORTA: %-5s \033[1;31m%s\033[0m\n" "$port"
        done < "$PORTS_FILE"
    fi

    echo -e "\033[0;34m--------------------------------------------------------------\033[0m"
    echo -e "\033[1;31m[\033[1;36m01\033[1;31m] \033[1;34m◉ \033[1;33mABRIR PORTAS \033[1;31m
[\033[1;36m02\033[1;31m] \033[1;34m◉ \033[1;33mFECHAR PORTAS \033[1;31m
[\033[1;36m03\033[1;31m] \033[1;34m◉ \033[1;33mALTERAR STATUS \033[1;31m
[\033[1;36m04\033[1;31m] \033[1;34m◉ \033[1;33mREMOVER SCRIPT \033[1;31m
[\033[1;36m00\033[1;31m] \033[1;34m◉ \033[1;33mSAIR DO MENU \033[1;31m"
    echo -e "\033[0;34m--------------------------------------------------------------\033[0m"
    echo
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
            add_proxy_port $port "$status"
            read -p "✅ PORTA ATIVADA COM SUCESSO. PRESSIONE QUALQUER TECLA PARA VOLTAR AO MENU." dummy
            ;;
        2)
            clear
            read -p "DIGITE A PORTA: " port
            while ! [[ $port =~ ^[0-9]+$ ]]; do
                echo "DIGITE UMA PORTA VÁLIDA."
                read -p "DIGITE A PORTA: " port
            done
            del_proxy_port $port
            read -p "✅ PORTA DESATIVADA. PRESSIONE QUALQUER TECLA PARA VOLTAR AO MENU." dummy
			clear
            ;;
        3)
            clear
            read -p "DIGITE A PORTA: " port
            while ! [[ $port =~ ^[0-9]+$ ]]; do
                echo "DIGITE UMA PORTA VÁLIDA."
                read -p "DIGITE A PORTA: " port
            done
            read -p "DIGITE O NOVO STATUS DE CONEXÃO: " new_status
            update_proxy_status $port "$new_status"
            read -p "✅ STATUS DA PORTA ATUALIZADO. PRESSIONE QUALQUER TECLA PARA VOLTAR AO MENU." dummy
            ;;
			
	4)
          clear
            uninstall_rustyproxy
            read -p "◉ PRESSIONE QUALQUER TC PARA SAIR." dummy
	    clear
            exit 0
            ;;	
			
        0)
            exit 0
            ;;
        *)
            echo "OPÇÃO INVÁLIDA. PRESSIONE QUALQUER TECLA PARA VOLTAR AO MENU."
            read -n 1 dummy
            ;;
    esac
}

# Verificar se o arquivo de portas existe, caso contrário, criar
if [ ! -f "$PORTS_FILE" ]; then
    sudo touch "$PORTS_FILE"
fi

# Loop do menu
while true; do
    show_menu
done
