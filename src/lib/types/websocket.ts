// WebSocket API type definitions

export interface WebSocketStatus {
  is_running: boolean;
  actual_port: number | null;
  connected_clients: number;
}

export interface WebSocketStartResult {
  actual_port: number;
}
