import { describe, it, expect } from "vitest";
import { parseParams, applyParams, pair } from "./types";
describe("URL 与参数同步", () => {
  it("保留重复参数和中文并只拼接一次", () => {
    const url = "https://example.com/api?a=1&a=2&q=%E4%B8%AD%E6%96%87#part";
    const params = parseParams(url);
    expect(params.map((p) => [p.key, p.value])).toEqual([
      ["a", "1"],
      ["a", "2"],
      ["q", "中文"],
    ]);
    expect(applyParams(url, params)).toBe(url);
    expect(applyParams(applyParams(url, params), params)).toBe(url);
  });
  it("禁用、删除和空值正确处理", () => {
    const params = parseParams("http://localhost/?a=1&b=2");
    params[0].enabled = false;
    params[1].value = "";
    expect(applyParams("http://localhost/?a=1&b=2", params)).toBe(
      "http://localhost/?b=",
    );
    expect(applyParams("http://localhost/?a=1#hash", [])).toBe(
      "http://localhost/#hash",
    );
  });
  it("正确编码空格、加号、等号和问号", () => {
    const params = [{ ...pair(), key: "a b", value: "a+b=?" }];
    expect(
      parseParams(applyParams("https://example.com", params))[0].value,
    ).toBe("a+b=?");
    expect(parseParams("https://example.com?x=a?b")[0].value).toBe("a?b");
  });
});
