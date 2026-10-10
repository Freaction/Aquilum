export function isMacOs(): boolean {
  return /mac|iphone|ipad|ipod/i.test(navigator.platform)
    || /mac os|macintosh/i.test(navigator.userAgent);
}
