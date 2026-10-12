/*
 * SCOTOMA research-only conformance probe for liboprf 0.9.4.
 *
 * This file is not product code and carries no operational key material.
 */
#include <sodium.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "oprf.h"
#include "toprf.h"

enum {
  PEERS = 3,
  THRESHOLD = 2,
};

static int combine_pair(const uint8_t responses[PEERS][TOPRF_Part_BYTES],
                        const size_t left, const size_t right,
                        uint8_t result[crypto_core_ristretto255_BYTES]) {
  uint8_t selected[THRESHOLD][TOPRF_Part_BYTES];
  memcpy(selected[0], responses[left], TOPRF_Part_BYTES);
  memcpy(selected[1], responses[right], TOPRF_Part_BYTES);
  return toprf_thresholdmult(THRESHOLD, selected, result);
}

static int evaluate_all(
    const uint8_t key_shares[PEERS][TOPRF_Share_BYTES],
    const uint8_t zero_shares[PEERS][TOPRF_Share_BYTES],
    const uint8_t alpha[crypto_core_ristretto255_BYTES], const uint8_t *ssid,
    const uint16_t ssid_len,
    uint8_t responses[PEERS][TOPRF_Part_BYTES]) {
  for (size_t i = 0; i < PEERS; i++) {
    if (toprf_3hashtdh(key_shares[i], zero_shares[i], alpha, ssid, ssid_len,
                      responses[i]) != 0) {
      return 1;
    }
  }
  return 0;
}

int main(void) {
  if (sodium_init() < 0) {
    fputs("libsodium initialization failed\n", stderr);
    return 1;
  }

  uint8_t key[crypto_core_ristretto255_SCALARBYTES] = {0};
  key[0] = 7;
  uint8_t zero[crypto_core_ristretto255_SCALARBYTES] = {0};
  uint8_t key_shares[PEERS][TOPRF_Share_BYTES];
  uint8_t zero_shares[PEERS][TOPRF_Share_BYTES];
  toprf_create_shares(key, PEERS, THRESHOLD, key_shares);
  toprf_create_shares(zero, PEERS, THRESHOLD, zero_shares);

  static const uint8_t credential[] =
      "scotoma-v3-test-only-room-password-credential";
  uint8_t blind[crypto_core_ristretto255_SCALARBYTES];
  uint8_t alpha[crypto_core_ristretto255_BYTES];
  if (oprf_Blind(credential, (uint16_t)(sizeof credential - 1), blind, alpha) !=
      0) {
    fputs("OPRF blinding failed\n", stderr);
    return 1;
  }

  static const uint8_t ssid_a[] =
      "SCOTOMA-v3|manifest=7f1d|epoch=19|evaluation=0001";
  static const uint8_t ssid_b[] =
      "SCOTOMA-v3|manifest=7f1d|epoch=19|evaluation=0002";
  uint8_t responses_a[PEERS][TOPRF_Part_BYTES];
  uint8_t responses_b[PEERS][TOPRF_Part_BYTES];
  if (evaluate_all(key_shares, zero_shares, alpha, ssid_a,
                   (uint16_t)(sizeof ssid_a - 1), responses_a) != 0 ||
      evaluate_all(key_shares, zero_shares, alpha, ssid_b,
                   (uint16_t)(sizeof ssid_b - 1), responses_b) != 0) {
    fputs("3HashTDH evaluation failed\n", stderr);
    return 1;
  }

  uint8_t expected[crypto_core_ristretto255_BYTES];
  if (oprf_Evaluate(key, alpha, expected) != 0) {
    fputs("base OPRF evaluation failed\n", stderr);
    return 1;
  }

  static const size_t pairs[][2] = {{0, 1}, {0, 2}, {1, 2}};
  uint8_t combined[crypto_core_ristretto255_BYTES];
  for (size_t i = 0; i < sizeof pairs / sizeof pairs[0]; i++) {
    if (combine_pair(responses_a, pairs[i][0], pairs[i][1], combined) != 0 ||
        sodium_memcmp(combined, expected, sizeof expected) != 0) {
      fputs("an honest 2-of-3 quorum diverged from the base OPRF\n", stderr);
      return 1;
    }
  }

  uint8_t mixed[THRESHOLD][TOPRF_Part_BYTES];
  memcpy(mixed[0], responses_a[0], TOPRF_Part_BYTES);
  memcpy(mixed[1], responses_b[1], TOPRF_Part_BYTES);
  if (toprf_thresholdmult(THRESHOLD, mixed, combined) != 0) {
    fputs("mixed-session combination returned an API error\n", stderr);
    return 1;
  }
  if (sodium_memcmp(combined, expected, sizeof expected) == 0) {
    fputs("mixed session identifiers unexpectedly produced the OPRF output\n",
          stderr);
    return 1;
  }

  if (toprf_thresholdmult(1, responses_a, combined) != 0) {
    fputs("single-share combination returned an API error\n", stderr);
    return 1;
  }
  if (sodium_memcmp(combined, expected, sizeof expected) == 0) {
    fputs("a single share unexpectedly produced the threshold output\n", stderr);
    return 1;
  }

  sodium_memzero(blind, sizeof blind);
  sodium_memzero(key, sizeof key);
  sodium_memzero(key_shares, sizeof key_shares);
  sodium_memzero(zero_shares, sizeof zero_shares);
  puts("{\"schema\":\"scotoma-liboprf-probe-v1\",\"honest_quorums\":3,"
       "\"mixed_session_rejected\":true,\"below_threshold_rejected\":true,"
       "\"result\":\"pass\"}");
  return 0;
}
