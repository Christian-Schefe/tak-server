import { useFetch } from '@/utils/fetch';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { computed, toValue, type MaybeRefOrGetter } from 'vue';
import { z } from 'zod';

const profileSchema = z.object({
  displayName: z.string().nullable(),
  country: z.string().nullable(),
  profilePictureVersion: z.number().nullable(),
});

export function useProfile(userId: MaybeRefOrGetter<string | undefined>) {
  const { fetchTyped } = useFetch();
  return useQuery({
    queryKey: ['profile', userId],
    queryFn: async () => {
      const id = toValue(userId) ?? '';
      return await fetchTyped(profileSchema, `/api/profiles/${id}`);
    },
    enabled: () => toValue(userId) !== undefined,
    staleTime: 1000 * 60 * 5,
  });
}

export function getProfilePictureUrl(userId: string, version: number) {
  return `/api/profiles/${userId}/picture?v=${version.toString()}`;
}

export function useProfilePictureUrl(userId: MaybeRefOrGetter<string | undefined>) {
  const { data: profile } = useProfile(userId);
  return computed(() => {
    const accId = toValue(userId);
    const val = profile.value;
    if (accId === undefined || !val || val.profilePictureVersion === null) {
      return '/fallback/default_user.webp';
    }
    return getProfilePictureUrl(accId, val.profilePictureVersion);
  });
}

export function useUploadProfilePicture() {
  const { fetch } = useFetch();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ file, userId }: { file: File; userId: string }) => {
      const formData = new FormData();
      formData.append('picture', file);
      await fetch(`/api/profiles/${userId}/picture`, 'POST', formData);
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['profile'] });
    },
  });
}

export interface AccountProfileUpdate {
  userId: string;
  country: string | null;
  displayName: string | null;
}

export function useUpdateProfile() {
  const queryClient = useQueryClient();
  const { fetch } = useFetch();
  return useMutation({
    mutationFn: async (update: AccountProfileUpdate) => {
      await fetch(`/api/profiles/${update.userId}`, 'POST', update);
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['profile'] });
    },
  });
}
