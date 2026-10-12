/* Deterministic TEST-ONLY 3HashTDH vector for cross-platform comparison. */
#include <sodium.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "oprf.h"
#include "toprf.h"

enum { PEERS = 3, THRESHOLD = 2 };

static void scalar_u8(uint8_t scalar[crypto_core_ristretto255_SCALARBYTES],
                      uint8_t value) {
  memset(scalar, 0, crypto_core_ristretto255_SCALARBYTES);
  scalar[0] = value;
}

static void make_linear_shares(
    uint8_t secret_value, uint8_t coefficient_value,
    uint8_t shares[PEERS][TOPRF_Share_BYTES]) {
  uint8_t secret[crypto_core_ristretto255_SCALARBYTES];
  uint8_t coefficient[crypto_core_ristretto255_SCALARBYTES];
  scalar_u8(secret, secret_value);
  scalar_u8(coefficient, coefficient_value);
  for (uint8_t i = 1; i <= PEERS; i++) {
    uint8_t x[crypto_core_ristretto255_SCALARBYTES];
    uint8_t product[crypto_core_ristretto255_SCALARBYTES];
    scalar_u8(x, i);
    crypto_core_ristretto255_scalar_mul(product, coefficient, x);
    shares[i - 1][0] = i;
    crypto_core_ristretto255_scalar_add(shares[i - 1] + 1, secret, product);
  }
}

static int combine_pair(const uint8_t responses[PEERS][TOPRF_Part_BYTES],
                        size_t left, size_t right,
                        uint8_t result[crypto_core_ristretto255_BYTES]) {
  uint8_t selected[THRESHOLD][TOPRF_Part_BYTES];
  memcpy(selected[0], responses[left], TOPRF_Part_BYTES);
  memcpy(selected[1], responses[right], TOPRF_Part_BYTES);
  return toprf_thresholdmult(THRESHOLD, selected, result);
}

static void print_hex(const uint8_t *bytes, size_t length) {
  for (size_t i = 0; i < length; i++) {
    printf("%02x", bytes[i]);
  }
}

int main(void) {
  if (sodium_init() < 0) return 1;

  static const uint8_t credential[] = "scotoma-v3-vector-room-password";
  static const uint8_t ssid[] =
      "SCOTOMA-v3|manifest=7f1d|epoch=19|evaluation=0001";
  uint8_t key[crypto_core_ristretto255_SCALARBYTES];
  uint8_t blind[crypto_core_ristretto255_SCALARBYTES];
  uint8_t key_shares[PEERS][TOPRF_Share_BYTES];
  uint8_t zero_shares[PEERS][TOPRF_Share_BYTES];
  scalar_u8(key, 7);
  scalar_u8(blind, 5);
  make_linear_shares(7, 11, key_shares);
  make_linear_shares(0, 13, zero_shares);

  uint8_t input_point[crypto_core_ristretto255_BYTES];
  uint8_t alpha[crypto_core_ristretto255_BYTES];
  if (voprf_hash_to_group(credential, (uint16_t)(sizeof credential - 1),
                          input_point) != 0 ||
      crypto_scalarmult_ristretto255(alpha, blind, input_point) != 0) {
    return 1;
  }

  uint8_t responses[PEERS][TOPRF_Part_BYTES];
  for (size_t i = 0; i < PEERS; i++) {
    if (toprf_3hashtdh(key_shares[i], zero_shares[i], alpha, ssid,
                      (uint16_t)(sizeof ssid - 1), responses[i]) != 0) {
      return 1;
    }
  }

  uint8_t expected[crypto_core_ristretto255_BYTES];
  if (oprf_Evaluate(key, alpha, expected) != 0) return 1;
  static const size_t pairs[][2] = {{0, 1}, {0, 2}, {1, 2}};
  uint8_t combined[crypto_core_ristretto255_BYTES];
  for (size_t i = 0; i < sizeof pairs / sizeof pairs[0]; i++) {
    if (combine_pair(responses, pairs[i][0], pairs[i][1], combined) != 0 ||
        sodium_memcmp(expected, combined, sizeof expected) != 0) {
      return 1;
    }
  }

  uint8_t unblinded[crypto_core_ristretto255_BYTES];
  uint8_t final_output[OPRF_BYTES];
  if (oprf_Unblind(blind, expected, unblinded) != 0 ||
      oprf_Finalize(credential, (uint16_t)(sizeof credential - 1), unblinded,
                    final_output) != 0) {
    return 1;
  }

  printf("{\"schema\":\"scotoma-3hashtdh-vector-v1\",\"id\":\"test-only-001\","
         "\"alpha\":\"");
  print_hex(alpha, sizeof alpha);
  printf("\",\"responses\":[");
  for (size_t i = 0; i < PEERS; i++) {
    if (i != 0) putchar(',');
    putchar('"');
    print_hex(responses[i], TOPRF_Part_BYTES);
    putchar('"');
  }
  printf("],\"beta\":\"");
  print_hex(expected, sizeof expected);
  printf("\",\"final\":\"");
  print_hex(final_output, sizeof final_output);
  puts("\"}");
  return 0;
}
