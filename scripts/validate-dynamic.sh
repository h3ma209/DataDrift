#!/usr/bin/env bash
# Phase-2 dynamic validation helpers for DataGrip (no secrets, read-only).
set -euo pipefail

GRIP_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
JAR_INDEX="${GRIP_ROOT}/tmp/jar-index-database-modules.txt"
JAR_INDEX_LIB="${GRIP_ROOT}/tmp/jar-index.txt"
SELECTOR="${IDEA_PATHS_SELECTOR:-DataGrip2026.1}"
LOG_DIR="${HOME}/Library/Logs/JetBrains/${SELECTOR}"
LOG_FILE="${LOG_DIR}/idea.log"

usage() {
  cat <<'EOF'
Usage: validate-dynamic.sh <command>

Commands:
  index <fqcn-substring>   Grep jar-index files for validation target classes
  logs [path]              Tail JDBC/RMI/introspection lines from idea.log
  jdbc-children            List RemoteJdbcServer child JVM processes + listen ports
  rmi-ports                Parse latest Port/ServicesPort/ID lines from idea.log
  all                      Run jdbc-children + rmi-ports + logs (last 30 matches)
EOF
}

cmd_index() {
  local q="${1:?pattern required}"
  for f in "$JAR_INDEX" "$JAR_INDEX_LIB"; do
    [[ -f "$f" ]] || continue
    echo "== $f =="
    rg -i "$q" "$f" || true
  done
}

cmd_logs() {
  local f="${1:-$LOG_FILE}"
  [[ -f "$f" ]] || { echo "No log: $f" >&2; exit 1; }
  rg -i 'RemoteProcessSupport|RemoteJdbcServer|DatabaseConnectionEstablisher|IntrospectionQueryContext|DatabaseModelLoader|DataSourceSync|ApplicationLoader|plugin initialization|Using classpath' "$f" | tail -40
}

cmd_jdbc_children() {
  pgrep -fl 'RemoteJdbcServer' || echo "(no RemoteJdbcServer JVMs running)"
  echo "--- listen sockets (RemoteJdbcServer PIDs) ---"
  while read -r pid _; do
    [[ -n "$pid" ]] || continue
    echo "PID $pid"
    lsof -nP -a -p "$pid" -iTCP -sTCP:LISTEN 2>/dev/null || true
  done < <(pgrep -f 'RemoteJdbcServer' || true)
}

cmd_rmi_ports() {
  [[ -f "$LOG_FILE" ]] || { echo "No log: $LOG_FILE" >&2; exit 1; }
  rg 'Port/ServicesPort/ID|Started remote process on' "$LOG_FILE" | tail -10
}

case "${1:-}" in
  index) shift; cmd_index "$@" ;;
  logs) shift; cmd_logs "$@" ;;
  jdbc-children) cmd_jdbc_children ;;
  rmi-ports) cmd_rmi_ports ;;
  all)
    cmd_jdbc_children
    echo
    cmd_rmi_ports
    echo
    cmd_logs
    ;;
  -h|--help|help|"") usage ;;
  *) echo "Unknown command: $1" >&2; usage; exit 1 ;;
esac
