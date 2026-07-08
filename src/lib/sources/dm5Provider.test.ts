import { describe, expect, it } from "vitest";

import { loadDm5Detail, parseDm5Detail, parseDm5SearchResults, searchDm5 } from "./dm5Provider";

describe("dm5 provider", () => {
  it("parses search results into shared comic summaries", () => {
    const html = `
      <div class="mh-item mh-card-wrap">
        <div class="mh-item-detali"><a href="/manhua-haizeiwang-onepiece/" title="海贼王">海贼王</a></div>
        <div class="manga-info" uk="manhua-haizeiwang-onepiece" tc="https://cover.test/one.jpg" ts="4" tt="相传22年前" au="尾田荣一郎,"></div>
      </div>
    `;

    expect(parseDm5SearchResults(html)).toEqual([
      {
        id: "dm5:/manhua-haizeiwang-onepiece/",
        title: "海贼王",
        author: "尾田荣一郎",
        thumb_url: "https://cover.test/one.jpg",
        categories: ["DM5"],
        likes_count: 0,
        total_views: 0,
        eps_count: 4,
        finished: false,
      },
    ]);
  });

  it("parses 1kkk search results with a separate provider prefix", () => {
    const html = `
      <div class="mh-item mh-card-wrap">
        <div class="mh-item-detali"><a href="/manhua81674/" title="反派美学">反派美学</a></div>
        <div class="manga-info" uk="manhua81674" tc="https://cover.test/fanpai.jpg" ts="1" tt="季詩瑄穿越成公主" lu="ch0-1330219" te="反派美学 第0话" tus="连载中" tn="第0话 " au="佚名,"></div>
      </div>
    `;

    expect(parseDm5SearchResults(html, "ikkk")[0]).toMatchObject({
      id: "ikkk:/manhua81674/",
      title: "反派美学",
      thumb_url: "https://cover.test/fanpai.jpg",
      categories: ["1kkk", "反派美学 第0话"],
      eps_count: 1,
      finished: false,
    });
  });

  it("parses details and chapter web urls", () => {
    const html = `
      <script>var DM5_COMIC_MNAME="海贼王";</script>
      <div class="banner_detail_form"><img src="https://cover.test/one.jpg"></div>
      <p class="content">大秘宝的故事</p>
      <a href="/m1799951/" title="海贼王 第1187话"></a>
      <a href="/m1/" title="海贼王 第1话"></a>
      <a href="/m1/" title="重复"></a>
    `;

    expect(parseDm5Detail(html, "/manhua-haizeiwang-onepiece/")).toMatchObject({
      detail: {
        id: "dm5:/manhua-haizeiwang-onepiece/",
        title: "海贼王",
        thumb_url: "https://cover.test/one.jpg",
        description: "大秘宝的故事",
        eps_count: 2,
      },
      chapters: [
        { id: "https://www.dm5.com/m1799951/", title: "海贼王 第1187话", order: 1, updated_at: "" },
        { id: "https://www.dm5.com/m1/", title: "海贼王 第1话", order: 2, updated_at: "" },
      ],
    });
  });

  it("parses chapters only from the detail chapter list", () => {
    const html = `
      <script>var DM5_COMIC_MNAME="反派美学";</script>
      <meta content="反派美学漫画简介：季詩瑄不僅穿到自己寫的小說中變成了白癡公主伊奧內。" name="Description" />
      <div class="detail-list-title">
        <a href="javascript:void(0);" class="block ">连载<span>（1）</span></a>
        <span class="s">最新<span>&nbsp;<a href="/m1330219/" title="反派美学 第0话">第0话 </a>&nbsp;2022-10-11 </span></span>
      </div>
      <div id="chapterlistload">
        <ul class="view-win-list detail-list-select" id="detail-list-select-1">
          <li><a href="/m1330219/" title="" target="_blank">第0话 <span>（21P）</span></a></li>
        </ul>
      </div>
      <div class="index-manga">
        <p class="chapter"><a href="/m202723/" title="反派灰姑娘 第5话">第5话</a></p>
      </div>
    `;

    expect(parseDm5Detail(html, "/manhua-fanpaimeixue/")).toMatchObject({
      detail: {
        id: "dm5:/manhua-fanpaimeixue/",
        title: "反派美学",
        description: "反派美学漫画简介：季詩瑄不僅穿到自己寫的小說中變成了白癡公主伊奧內。",
        eps_count: 1,
        categories: ["DM5", "连载（1）", "最新 第0话 2022-10-11"],
      },
      chapters: [
        { id: "https://www.dm5.com/m1330219/", title: "第0话", order: 1, updated_at: "" },
      ],
    });
  });

  it("uses the injected text fetcher for search and detail", async () => {
    await expect(searchDm5(async () => "", "one piece")).resolves.toEqual([]);
    await expect(loadDm5Detail(async () => "<script>var DM5_COMIC_MNAME=\"测试\";</script>", "dm5:/manhua-test/")).resolves.toMatchObject({
      detail: { title: "测试" },
    });
  });

  it("uses 1kkk host for injected search and details", async () => {
    const seenUrls: string[] = [];
    const fetchText = async (url: string) => {
      seenUrls.push(url);
      return "<script>var DM5_COMIC_MNAME=\"测试\";</script><div id=\"chapterlistload\"><a href=\"/ch1-1/\" title=\"第 1 话\">第 1 话</a></div>";
    };

    await searchDm5(fetchText, "one piece", "ikkk");
    await expect(loadDm5Detail(fetchText, "ikkk:/manhua81674/")).resolves.toMatchObject({
      detail: { id: "ikkk:/manhua81674/", title: "测试", categories: ["1kkk"] },
      chapters: [{ id: "https://www.1kkk.com/ch1-1/" }],
    });
    expect(seenUrls).toEqual([
      "https://www.1kkk.com/search?title=one%20piece&language=1",
      "https://www.1kkk.com/manhua81674/",
    ]);
  });
});
