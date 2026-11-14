import { expect, test } from "vitest";

import { confirmationProblem, emailProblem, passwordProblem } from "./validation";

test("emailProblem accepts ordinary addresses", () => {
  expect(emailProblem("reader@example.com")).toBeNull();
  expect(emailProblem("a.b+tag@sub.example.co.uk")).toBeNull();
});

test("emailProblem rejects what the server rejects", () => {
  expect(emailProblem("")).toBe("Enter your email");
  expect(emailProblem("reader")).not.toBeNull();
  expect(emailProblem("reader@example")).not.toBeNull();
  expect(emailProblem("reader @example.com")).not.toBeNull();
  expect(emailProblem("a@b@c.com")).not.toBeNull();
});

test("passwordProblem enforces the server's minimum", () => {
  expect(passwordProblem("short")).toMatch(/at least 8/i);
  expect(passwordProblem("long-enough")).toBeNull();
});

test("confirmationProblem catches typos", () => {
  expect(confirmationProblem("long-enough", "long-enoug")).toMatch(/do not match/i);
  expect(confirmationProblem("long-enough", "long-enough")).toBeNull();
});
