// 認証関連の Tauri コマンドラッパー

import { invoke } from '@tauri-apps/api/core';
import type { AuthStatus, SessionValidity } from '$lib/types';
import { normalizeError } from './errors';

export async function authGetStatus(): Promise<AuthStatus> {
  try {
    return await invoke('auth_get_status');
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function authLoadCredentials(): Promise<boolean> {
  try {
    return await invoke('auth_load_credentials');
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function authSaveRawCookies(rawCookies: string): Promise<void> {
  try {
    await invoke('auth_save_raw_cookies', { rawCookies });
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function authSaveCredentials(
  sid: string,
  hsid: string,
  ssid: string,
  apisid: string,
  sapisid: string
): Promise<void> {
  try {
    await invoke('auth_save_credentials', { sid, hsid, ssid, apisid, sapisid });
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function authDeleteCredentials(): Promise<void> {
  try {
    await invoke('auth_delete_credentials');
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function authClearWebviewCookies(): Promise<void> {
  try {
    await invoke('auth_clear_webview_cookies');
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function authValidateCredentials(): Promise<boolean> {
  try {
    return await invoke('auth_validate_credentials');
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function authCheckSessionValidity(): Promise<SessionValidity> {
  try {
    return await invoke('auth_check_session_validity');
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function authUseFallbackStorage(): Promise<boolean> {
  try {
    return await invoke('auth_use_fallback_storage');
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function authOpenWindow(): Promise<void> {
  try {
    await invoke('auth_open_window');
  } catch (e) {
    throw normalizeError(e);
  }
}
