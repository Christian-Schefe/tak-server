import type { FormValidator, FormValidatorResult } from '@tak-ui-lib/components';
import { z } from 'zod';

export function zodFormValidator<T>(schema: z.ZodType<T>): FormValidator<T> {
  function validate(values: Record<string, unknown>): FormValidatorResult<T> {
    const result = schema.safeParse(values);
    if (result.success) {
      return { type: 'success', data: result.data };
    } else {
      const errors: Record<string, string> = {};
      for (const issue of result.error.issues) {
        errors[issue.path.join('.')] = issue.message;
      }
      return { type: 'error', errors };
    }
  }
  return validate;
}
