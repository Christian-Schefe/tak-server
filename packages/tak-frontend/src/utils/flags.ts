import * as flags from 'country-flag-icons/string/3x2';
import { computed, toValue, type MaybeRefOrGetter } from 'vue';
import { countries } from 'countries-list';
import DOMPurify from 'dompurify';

export const flagsMap = new Map<string, string>(Object.entries(flags));

export function useSanitizedFlagSVG(country: MaybeRefOrGetter<string | undefined>) {
  return computed(() => {
    const countryCode = toValue(country)?.toUpperCase();
    if (countryCode === undefined) {
      return undefined;
    }
    const svg = flagsMap.get(countryCode);
    if (svg === undefined) {
      console.warn(`No flag found for country code: ${countryCode}`);
      return undefined;
    }
    return DOMPurify.sanitize(svg, { USE_PROFILES: { svg: true } });
  });
}

export const countryArray = Object.entries(countries)
  .map(([code, country]) => ({
    code,
    name: country.name,
  }))
  .sort((a, b) => a.name.localeCompare(b.name));
