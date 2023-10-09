#include <algorithm>
#include <iostream>
#include <unordered_set>
#include <vector>

class Solution {
public:
  bool containsDuplicate(std::vector<int> &nums) {
    if (nums.size() < 2) {
      return false;
    }

    std::sort(nums.begin(), nums.end());

    int prev = nums[0];
    for (int i = 1; i < nums.size(); i++) {
      if (nums[i] == prev) {
        return true;
      }

      prev = nums[i];
    }

    return false;
  }
};

int main() {
  Solution s;
  std::vector<int> nums = {1, 2, 1, 0};
  bool result = s.containsDuplicate(nums);
  std::cout << "Solution: " << result << "\n";
  return 0;
}
