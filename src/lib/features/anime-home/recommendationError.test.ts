import { describe, expect, it } from "vitest";
import { friendlyRecommendationError } from "./recommendationError";

describe("friendly recommendation errors", () => {
  it("hides transport URLs and native connection details", () => {
    const message = friendlyRecommendationError(
      new Error("error sending request for url (https://api.example.test/search): connection reset by peer"),
      "请检查网络或规则源后重试",
    );

    expect(message).toBe("推荐源暂时无法连接，请检查网络或稍后重试");
    expect(message).not.toContain("https://");
  });

  it("normalizes parser errors while retaining a recoverable fallback", () => {
    expect(friendlyRecommendationError(new TypeError("Cannot read properties of null"), "请稍后重试"))
      .toBe("请稍后重试（源数据格式暂不可用）");
  });
});
