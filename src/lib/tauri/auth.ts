// Tauri Auth API wrapper

import { invoke } from '@tauri-apps/api/core';
import type { AuthStatus, SessionValidity } from '$lib/types';

export async function authGetStatus(): Promise<AuthStatus> {
  return invoke('auth_get_status');
}

export async function authLoadCredentials(): Promise<boolean> {
  return invoke('auth_load_credentials');
}

export async function authSaveRawCookies(rawCookies: string): Promise<void> {
  await invoke('auth_save_raw_cookies', { rawCookies });
}

export async function authSaveCredentials(
  sid: string,
  hsid: string,
  ssid: string,
  apisid: string,
  sapisid: string
): Promise<void> {
  await invoke('auth_save_credentials', { sid, hsid, ssid, apisid, sapisid });
}

export async function authDeleteCredentials(): Promise<void> {
  await invoke('auth_delete_credentials');
}

export async function authValidateCredentials(): Promise<boolean> {
  return invoke('auth_validate_credentials');
}

export async function authCheckSessionValidity(): Promise<SessionValidity> {
  return invoke('auth_check_session_validity');
}

export async function authUseFallbackStorage(): Promise<boolean> {
  return invoke('auth_use_fallback_storage');
}

export async function authOpenWindow(): Promise<void> {
  await invoke('auth_open_window');
}

// Deprecated - kept for backward compatibility
export async function authGetCredentialsPath(): Promise<string> {
  console.warn('authGetCredentialsPath is deprecated. Use authGetStatus().storage_type instead.');
  return invoke('auth_get_credentials_path');
}
