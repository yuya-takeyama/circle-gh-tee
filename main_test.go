package main

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestGetPrNumberFromEnv(t *testing.T) {
	num, err := getPrNumberFromEnv("https://github.com/user/repo/pull/123")

	assert.Equal(t, 123, num)
	assert.Nil(t, err)
}

func TestGetPrNumberFromEnvError(t *testing.T) {
	num, err := getPrNumberFromEnv("invalid input")

	assert.Equal(t, -1, num)
	assert.NotNil(t, err)
}
