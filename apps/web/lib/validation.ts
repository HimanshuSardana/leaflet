/** Client-side mirrors of the server's rules, so a form never round-trips to
 *  learn something it could have said locally. The server stays authoritative. */

export const MIN_PASSWORD = 8;

export function emailProblem(value: string): string | null {
  const email = value.trim();
  if (!email) return "Enter your email";
  const parts = email.split("@");
  if (parts.length !== 2) return "That does not look like an email";
  const [local, domain] = parts as [string, string];
  if (!local || !domain.includes(".") || domain.startsWith(".") || domain.endsWith(".")) {
    return "That does not look like an email";
  }
  if (/\s/.test(email)) return "Emails cannot contain spaces";
  return null;
}

export function passwordProblem(value: string): string | null {
  if (!value) return "Choose a password";
  if (value.length < MIN_PASSWORD) return `At least ${MIN_PASSWORD} characters`;
  return null;
}

export function confirmationProblem(password: string, confirmation: string): string | null {
  if (!confirmation) return "Repeat your password";
  if (password !== confirmation) return "Passwords do not match";
  return null;
}

export function displayNameProblem(value: string): string | null {
  const name = value.trim();
  if (!name) return "Pick a name";
  if (name.length > 64) return "At most 64 characters";
  return null;
}
