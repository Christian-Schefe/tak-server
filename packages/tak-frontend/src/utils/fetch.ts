import { useAuthStore } from '@/features/auth';
import type z from 'zod';

export function useFetch() {
  const authStore = useAuthStore();

  async function fetch2(
    path: string,
    method: 'GET' | 'POST' | 'DELETE' = 'GET',
    body?: unknown,
    doRefresh: boolean = true,
  ): Promise<Response> {
    const headers: HeadersInit = {};
    const authData = await authStore.getAuthData(doRefresh);
    if (authData) {
      headers['Authorization'] = `Bearer ${authData.jwt}`;
    }

    if (method === 'POST' && body !== undefined && !(body instanceof FormData)) {
      headers['Content-Type'] = 'application/json';
    }
    let response: Response;
    if (method === 'POST') {
      response = await fetch(path, {
        method: 'POST',
        headers,
        body:
          body !== undefined ? (body instanceof FormData ? body : JSON.stringify(body)) : undefined,
      });
    } else {
      response = await fetch(path, {
        method,
        headers,
      });
    }
    if (!response.ok) {
      if (response.status === 401) {
        authStore.setAuthData(null);
      }
      throw new Error(
        `Failed to fetch ${path}: ${response.status.toString()} ${response.statusText}`,
      );
    }
    return response;
  }

  async function fetchTyped<T>(
    schema: z.ZodType<T>,
    path: string,
    method: 'GET' | 'POST' | 'DELETE' = 'GET',
    body?: unknown,
    doRefresh: boolean = true,
  ): Promise<T> {
    const response = await fetch2(path, method, body, doRefresh);
    const data = await response.json();
    const parsed = schema.safeParse(data);
    if (!parsed.success) {
      console.error(`Failed to parse response from ${path}:`, parsed.error);
      throw new Error(`Failed to parse response from ${path}: ${parsed.error}`);
    }
    return parsed.data;
  }

  return {
    fetch: fetch2,
    fetchTyped: fetchTyped,
  };
}
