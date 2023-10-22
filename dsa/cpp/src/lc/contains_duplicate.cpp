#include <algorithm>
#include <cassert>
#include <print>
#include <span>

class Solution {
public:
  bool containsDuplicate(std::span<int> nums) {
    if (nums.size() < 2) {
      return false;
    }

    // VSCode complains if I use std::ranges::sort :(
    std::sort(nums.begin(), nums.end());

    int prev = nums[0];
    for (const auto num : nums.subspan(1)) {
      if (num == prev) {
        return true;
      }

      prev = num;
    }

    return false;
  }
};

int main() {
  Solution s;
  {
    auto input = std::array{1, 2, 1, 0};
    auto result = s.containsDuplicate(input);
    assert(result == true);
    std::println("Solution 1: {0}", result);
  }
  {
    auto input = std::array{2, 1, 0};
    auto result = s.containsDuplicate(input);
    assert(result == false);
    std::println("Solution 2: {0}", result);
  }
  return 0;
}
