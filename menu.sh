#!/bin/bash

PORTS_FILE="/opt/rustyproxy/ports"

# Função para verificar se uma porta está em uso
is_port_in_use() {
    local port=$1
    
    if netstat -tuln 2>/dev/null | grep -q ":[0-9]*$port\b"; then
        return 0  
    elif ss -tuln 2>/dev/null | grep -q ":[0-9]*$port\b"; then
        return 0  
    else
        return 1 
    fi
}


# Função para abrir uma porta de proxy
add_proxy_port() {
    local port=$1
    local status=${2:-"@RustyProxy"}

    if is_port_in_use $port; then
        echo "A porta $port já está em uso."
        return
    fi

    local command="/opt/rustyproxy/proxy --port $port --status $status"
    local service_file_path="/etc/systemd/system/proxy${port}.service"
    local service_file_content="[Unit]
Description=RustyProxy${port}
After=network.target

[Service]
LimitNOFILE=infinity
LimitNPROC=infinity
LimitMEMLOCK=infinity
LimitSTACK=infinity
LimitCORE=0
LimitAS=infinity
LimitRSS=infinity
LimitCPU=infinity
LimitFSIZE=infinity
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
    echo $port >> "$PORTS_FILE"
    echo "Porta $port ABERTA COM SUCESSO."
}

# Função para fechar uma porta de proxy
del_proxy_port() {
    local port=$1

    sudo systemctl disable "proxy${port}.service"
    sudo systemctl stop "proxy${port}.service"
    sudo rm -f "/etc/systemd/system/proxy${port}.service"
    sudo systemctl daemon-reload

    # Remover a porta do arquivo
    sed -i "/^$port$/d" "$PORTS_FILE"
    echo "Porta $port FECHADA COM SUCESSO."
    clear
}

# Função para desinstalar o RustyProxy
uninstall_rustyproxy() {
    echo "DESINSTALANDO RUSTY PROXY, AGUARDE..."
    sleep 4
    clear

# Parar e remover todos os serviços
    if [ -s "$PORTS_FILE" ]; then
        while read -r port; do
            del_proxy_port $port
        done < "$PORTS_FILE"
    fi
	
	# Remover binário, arquivos e diretórios
    sudo rm -rf /opt/rustyproxy
    sudo rm -f "$PORTS_FILE"

    echo -e "\033[0;34m---------------------------------------------------------\033[0m"
    echo -e "\E[44;1;37m           RUSTY PROXY DESINSTALADO COM SUCESSO.          \E[0m"
    echo -e "\033[0;34m---------------------------------------------------------\033[0m"
    sleep 4
    clear
}

# Função para exibir o menu formatado
show_menu() {
    clear
    echo -e "\033[0;34m--------------------------------------------------------------\033[0m"
    echo -e "\E[44;1;37m                   ⚒ RUSTY PROXY MANAGER ⚒                   \E[0m"
    echo -e "\033[0;34m--------------------------------------------------------------\033[0m"
    
    # Verifica se há portas ativas
    if [ ! -s "$PORTS_FILE" ]; then
        printf " PORTA ATIVA(s): %-34s\n" "NENHUMA"
    else
        active_ports=""
        while read -r port; do
            active_ports+=" $port"
        done < "$PORTS_FILE"
        printf " PORTA(s):%-35s\n" "$active_ports"
    fi

    echo -e "\033[0;34m--------------------------------------------------------------\033[0m"
    echo -e "\033[1;31m[\033[1;36m01\033[1;31m] \033[1;34m◉ \033[1;33mABRIR PORTAS \033[1;31m
[\033[1;36m02\033[1;31m] \033[1;34m◉ \033[1;33mFECHAR PORTAS \033[1;31m
[\033[1;36m03\033[1;31m] \033[1;34m◉ \033[1;33mFECHAR PORTAS \033[1;31m
[\033[1;36m04\033[1;31m] \033[1;34m◉ \033[1;33mDESINSTALAR RUSTY PROXY \033[1;31m
[\033[1;36m00\033[1;31m] \033[1;37m\033[1;34m◉ \033[1;33mSAIR DO MENU \033[1;31m"
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
            read -p "DIGITE O STATUS DE CONEXÃO (DEIXE VAZIO PARA PADRÃO): " status
            add_proxy_port $port "$status"
			clear
            read -p "✅ PORTA ATIVADA COM SUCESSO. PRESSIONE QUALQUER TC PARA VOLTAR AO MENU." dummy
            ;;
        2)
		    clear
            read -p "DIGITE A PORTA: " port
            while ! [[ $port =~ ^[0-9]+$ ]]; do
                echo "DIGITE UMA PORTA VÁLIDA."
                read -p "DIGITE A PORTA: " port
		done
            del_proxy_port $port
	    clear
            read -p "✅ PORTA DESATIVADA. PRESSIONE QUALQUER TC PARA VOLTAR AO MENU." dummy
	    clear
            ;;
		3)
		  read -p "ALTERAR STATUS RUSTY PROXYS: " status
            add_proxy_port $port "$status"
		 read -p "✅ STATUS ALTERADO COM SUCESSO. PRESSIONE QUALQUER TC PARA VOLTAR AO MENU." dummy	
		 clear	
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
	    clear
            ;;
        *)
            echo "OPÇÃO INVÁLIDA.´PRESSIONE QUALQUER TC PARA VOLTAR AO MENU. inválida."
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
