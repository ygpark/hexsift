#!/bin/bash

echo "=== hexsift 테스트 스위트 ==="
echo

# 디버그 빌드
echo "1. 디버그 빌드 중..."
cargo build

if [ $? -ne 0 ]; then
    echo "빌드 실패!"
    exit 1
fi

# 유닛 테스트 실행
echo
echo "2. 유닛 테스트 실행 중..."
cargo test --lib

if [ $? -ne 0 ]; then
    echo "유닛 테스트 실패!"
    exit 1
fi

# 통합 테스트 실행
echo
echo "3. 통합 테스트 실행 중..."
cargo test --test integration_test

if [ $? -ne 0 ]; then
    echo "통합 테스트 실패!"
    exit 1
fi

# 테스트 데이터 생성기 테스트
echo
echo "4. 테스트 데이터 생성기 테스트 실행 중..."
cargo test --test test_data_generator

if [ $? -ne 0 ]; then
    echo "테스트 데이터 생성기 테스트 실패!"
    exit 1
fi

# 릴리즈 빌드 (벤치마크용)
echo
echo "5. 릴리즈 빌드 중..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "릴리즈 빌드 실패!"
    exit 1
fi

echo
echo "=== 모든 테스트 통과! ==="


echo "테스트 커버리지 확인 방법:"
echo "cargo install cargo-tarpaulin"
echo "cargo tarpaulin --out Html"