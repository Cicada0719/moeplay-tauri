import { describe, expect, it } from "vitest";
import {
  loadBaoziChapterImages,
  loadBaoziDetail,
  normalizeBaoziImageUrl,
  parseBaoziChapterPage,
  parseBaoziDetail,
  parseBaoziSearchResults,
  searchBaozi,
} from "./baoziProvider";

const searchHtml = `
  <div class="classify-items">
    <div>
      <a class="comics-card__poster" href="/comic/test-manga"><amp-img src="//img.test/cover.jpg"></amp-img><div class="tabs"><span class="tab">连载中</span></div></a>
      <div class="comics-card__info"><div class="comics-card__title">测试漫画</div><div class="tags">测试作者</div></div>
    </div>
  </div>
`;

const detailHtml = `
  <meta name="og:url" content="https://www.baozimh.com/comic/canonical-id" />
  <meta name="description" content="测试简介" />
  <div class="comics-detail"><div class="l-content">
    <amp-img src="//img.test/detail.jpg"></amp-img>
    <div class="comics-detail__title">测试漫画</div>
    <div class="comics-detail__author">测试作者</div>
    <div class="tag-list"><span class="tag">热血</span><span class="tag">已完结</span></div>
  </div></div>
  <div id="chapter-items">
    <div><a href="/user/page_direct?comic_id=canonical-id&amp;section_slot=0&amp;chapter_slot=2"><span>第2话</span></a></div>
    <div><a href="/user/page_direct?comic_id=canonical-id&amp;section_slot=0&amp;chapter_slot=1"><span>第1话</span></a></div>
  </div>
`;

describe("baozi provider", () => {
  it("rewrites the blocked Baozi image CDN to the working mirror", () => {
    expect(normalizeBaoziImageUrl(
      "https://static-tw.baozimh.com/cover/test.jpg?w=285&h=375&q=100",
    )).toBe("https://static-tw.baozimhcn.com/cover/test.jpg?w=285&h=375&q=100");
    expect(normalizeBaoziImageUrl("//static-tw.baozimh.com/cover/test.jpg"))
      .toBe("https://static-tw.baozimhcn.com/cover/test.jpg");
    expect(normalizeBaoziImageUrl("https://img.test/cover.jpg"))
      .toBe("https://img.test/cover.jpg");
  });

  it("normalizes Baozi cover URLs from search and detail pages", () => {
    const blockedSearchHtml = searchHtml.replace(
      "//img.test/cover.jpg",
      "https://static-tw.baozimh.com/cover/test.jpg?w=285&amp;h=375&amp;q=100",
    );
    const blockedDetailHtml = detailHtml.replace(
      "//img.test/detail.jpg",
      "https://static-tw.baozimh.com/cover/detail.jpg?w=285&amp;h=375&amp;q=100",
    );

    expect(parseBaoziSearchResults(blockedSearchHtml)[0]?.thumb_url)
      .toBe("https://static-tw.baozimhcn.com/cover/test.jpg?w=285&h=375&q=100");
    expect(parseBaoziDetail(blockedDetailHtml, "test-manga").detail.thumb_url)
      .toBe("https://static-tw.baozimhcn.com/cover/detail.jpg?w=285&h=375&q=100");
  });

  it("parses search cards into unified summaries", () => {
    expect(parseBaoziSearchResults(searchHtml)).toEqual([
      {
        id: "baozi:test-manga",
        title: "测试漫画",
        author: "测试作者",
        thumb_url: "https://img.test/cover.jpg",
        categories: ["包子漫画", "连载中"],
        likes_count: 0,
        total_views: 0,
        eps_count: 0,
        finished: false,
      },
    ]);
  });

  it("parses detail and canonical chapter urls", () => {
    const result = parseBaoziDetail(detailHtml, "test-manga");
    expect(result.detail).toMatchObject({
      id: "baozi:test-manga",
      title: "测试漫画",
      author: "测试作者",
      description: "测试简介",
      finished: true,
      eps_count: 2,
    });
    expect(result.chapters).toEqual([
      {
        id: "https://cn.dzmanga.com/comic/chapter/canonical-id/0_1_1.html",
        title: "第1话",
        order: 1,
        updated_at: "",
      },
      {
        id: "https://cn.dzmanga.com/comic/chapter/canonical-id/0_2_1.html",
        title: "第2话",
        order: 2,
        updated_at: "",
      },
    ]);
  });

  it("parses chapter images and follows the next-page link", () => {
    const html = `
      <div class="comic-contain"><div><amp-img src="//static-tw.baozimh.com/comic/id/1.jpg"></amp-img></div></div>
      <div class="next_chapter"><a href="/comic/chapter/id/0_1_2.html">下一页</a></div>
    `;
    expect(parseBaoziChapterPage(html, "https://cn.dzmanga.com/comic/chapter/id/0_1_1.html")).toEqual({
      images: ["https://static-tw.baozimhcn.com/comic/id/1.jpg"],
      nextUrl: "https://cn.dzmanga.com/comic/chapter/id/0_1_2.html",
    });
  });

  it("uses injected fetchers for search, detail, and paged images", async () => {
    await expect(searchBaozi(async () => searchHtml, "测试")).resolves.toHaveLength(1);
    await expect(loadBaoziDetail(async () => detailHtml, "baozi:test-manga")).resolves.toMatchObject({
      detail: { title: "测试漫画" },
    });

    const pages = new Map([
      [
        "https://cn.dzmanga.com/comic/chapter/id/0_1_1.html",
        `<div class="comic-contain"><div><amp-img src="https://img.test/1.jpg"></amp-img></div></div><div class="next_chapter"><a href="/comic/chapter/id/0_1_2.html">下一页</a></div>`,
      ],
      [
        "https://cn.dzmanga.com/comic/chapter/id/0_1_2.html",
        `<div class="comic-contain"><div><amp-img src="https://img.test/2.jpg"></amp-img></div></div>`,
      ],
    ]);
    await expect(loadBaoziChapterImages(async (url) => pages.get(url) || "", pages.keys().next().value!)).resolves.toEqual([
      { id: "baozi-page-1", url: "https://img.test/1.jpg" },
      { id: "baozi-page-2", url: "https://img.test/2.jpg" },
    ]);
  });
});
