import http from "k6/http";
import { sleep } from "k6";
import { expect } from "https://jslib.k6.io/k6-testing/0.5.0/index.js";

export const options = {
  vus: 10,
  duration: "30s",
};

export default function test() {
  const url = "http://localhost:7878";
  http.get(url);
  // sleep(1);
}
