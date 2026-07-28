import { useFetch } from '@/utils/fetch';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { computed, toValue, type MaybeRefOrGetter } from 'vue';
import { z } from 'zod';

const profileSchema = z.object({
  country: z.string().nullable(),
  profilePictureVersion: z.number().nullable(),
});

export function useProfile(accountId: MaybeRefOrGetter<string | undefined>) {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['profile', accountId],
    queryFn: async () => {
      const id = toValue(accountId) ?? '';
      return await fetchTyped(profileSchema, `/api/profiles/${id}`);
    },
    enabled: () => toValue(accountId) !== undefined,
    staleTime: 1000 * 60 * 5,
  });
}

export function getProfilePictureUrl(accountId: string, version: number) {
  return `/api/profiles/${accountId}/picture?v=${version.toString()}`;
}

export function useProfilePictureUrl(accountId: MaybeRefOrGetter<string | undefined>) {
  const { data: profile } = useProfile(accountId);
  return computed(() => {
    const accId = toValue(accountId);
    if (accId === undefined) {
      return undefined;
    }
    const val = profile.value;
    if (!val) {
      return undefined;
    }
    if (val.profilePictureVersion === null) {
      return '/fallback/default_user.webp';
    }
    return getProfilePictureUrl(accId, val.profilePictureVersion);
  });
}

export function useUploadProfilePicture() {
  const { fetch } = useFetch();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ file, accountId }: { file: File; accountId: string }) => {
      const formData = new FormData();
      formData.append('picture', file);
      await fetch(`/api/profiles/${accountId}/picture`, 'POST', formData);
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['profile'] });
    },
  });
}

export interface AccountProfileUpdate {
  accountId: string;
  country: string | null;
}

export function useUpdateProfile() {
  const queryClient = useQueryClient();
  const { fetch } = useFetch();
  return useMutation({
    mutationFn: async (update: AccountProfileUpdate) => {
      await fetch(`/api/profiles/${update.accountId}`, 'POST', update);
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['profile'] });
    },
  });
}
